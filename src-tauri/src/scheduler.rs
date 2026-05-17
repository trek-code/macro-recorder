use crate::types::ScheduledTask;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::interval;
use uuid::Uuid;

static TASKS: Lazy<Mutex<Vec<ScheduledTask>>> = Lazy::new(|| Mutex::new(Vec::new()));
static SCHEDULER_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn tasks_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("MacroRecorder").join("scheduled_tasks.json")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn load_from_disk() {
    if let Ok(content) = fs::read_to_string(tasks_path()) {
        if let Ok(tasks) = serde_json::from_str::<Vec<ScheduledTask>>(&content) {
            *TASKS.lock().unwrap() = tasks;
        }
    }
}

fn save_to_disk() {
    let tasks = TASKS.lock().unwrap().clone();
    if let Ok(json) = serde_json::to_string_pretty(&tasks) {
        if let Some(parent) = tasks_path().parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(tasks_path(), json).ok();
    }
}

/// Parse a simple cron-like expression.
/// Supports: "every_N_seconds", "daily_HH:MM", "once_MS", or full 5-field cron.
fn should_run_now(schedule: &str, last_run_ms: Option<u64>) -> bool {
    let now = now_ms();

    if let Some(s) = schedule.strip_prefix("every_") {
        if let Some(s2) = s.strip_suffix("_seconds") {
            if let Ok(secs) = s2.parse::<u64>() {
                let interval_ms = secs * 1000;
                return match last_run_ms {
                    None => true,
                    Some(last) => now.saturating_sub(last) >= interval_ms,
                };
            }
        }
        if let Some(s2) = s.strip_suffix("_minutes") {
            if let Ok(mins) = s2.parse::<u64>() {
                let interval_ms = mins * 60_000;
                return match last_run_ms {
                    None => true,
                    Some(last) => now.saturating_sub(last) >= interval_ms,
                };
            }
        }
    }

    if let Some(s) = schedule.strip_prefix("once_") {
        if let Ok(target_ms) = s.parse::<u64>() {
            return last_run_ms.is_none() && now >= target_ms;
        }
    }

    if let Some(s) = schedule.strip_prefix("daily_") {
        // Format: daily_HH:MM
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(h), Ok(m)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                let target_secs_since_midnight = h * 3600 + m * 60;
                let now_secs = now / 1000;
                let midnight_today = (now_secs / 86400) * 86400;
                let target_today = midnight_today + target_secs_since_midnight;
                let target_ms = target_today * 1000;
                if now >= target_ms && now < target_ms + 60_000 {
                    // Within the target minute
                    return match last_run_ms {
                        None => true,
                        Some(last) => last < target_ms,
                    };
                }
            }
        }
    }

    false
}

async fn run_task(task: &ScheduledTask) {
    let macro_name = task.macro_name.clone();
    let speed = task.speed;
    let loops = task.loops;

    tokio::task::spawn_blocking(move || {
        use crate::player;
        use crate::types::Macro;
        use std::fs;
        use dirs;

        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        let safe: String = macro_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let path = base.join("MacroRecorder").join("macros").join(format!("{}.json", safe));
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(m) = serde_json::from_str::<Macro>(&content) {
                let _ = player::play_macro_blocking(m, speed, loops, String::new());
            }
        }
    });
}

pub fn start_scheduler() {
    if SCHEDULER_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    load_from_disk();

    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            let tasks = TASKS.lock().unwrap().clone();
            for task in &tasks {
                if !task.enabled { continue; }
                if should_run_now(&task.schedule, task.last_run_ms) {
                    let task_clone = task.clone();
                    run_task(&task_clone).await;
                    // Update last_run
                    let mut all = TASKS.lock().unwrap();
                    if let Some(t) = all.iter_mut().find(|t| t.id == task_clone.id) {
                        t.last_run_ms = Some(now_ms());
                    }
                    drop(all);
                    save_to_disk();
                }
            }
        }
    });
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_scheduled_tasks() -> Vec<ScheduledTask> {
    load_from_disk();
    TASKS.lock().unwrap().clone()
}

#[tauri::command]
pub fn add_scheduled_task(
    macro_name: String,
    schedule: String,
    speed: f64,
    loops: u32,
) -> ScheduledTask {
    let task = ScheduledTask {
        id: Uuid::new_v4().to_string(),
        macro_name,
        schedule,
        speed,
        loops,
        enabled: true,
        last_run_ms: None,
    };
    TASKS.lock().unwrap().push(task.clone());
    save_to_disk();
    task
}

#[tauri::command]
pub fn toggle_scheduled_task(id: String, enabled: bool) -> Result<(), String> {
    let mut tasks = TASKS.lock().unwrap();
    let task = tasks.iter_mut().find(|t| t.id == id).ok_or("Task not found")?;
    task.enabled = enabled;
    drop(tasks);
    save_to_disk();
    Ok(())
}

#[tauri::command]
pub fn delete_scheduled_task(id: String) -> Result<(), String> {
    let mut tasks = TASKS.lock().unwrap();
    let pos = tasks.iter().position(|t| t.id == id).ok_or("Task not found")?;
    tasks.remove(pos);
    drop(tasks);
    save_to_disk();
    Ok(())
}

#[tauri::command]
pub async fn run_task_now(id: String) -> Result<(), String> {
    let task = TASKS
        .lock()
        .unwrap()
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or("Task not found")?;
    run_task(&task).await;
    Ok(())
}
