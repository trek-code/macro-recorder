use crate::image_match;
use crate::variables;
use crate::window;
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use once_cell::sync::Lazy;
use rhai::{Engine, Scope};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Script output log shared with frontend
static SCRIPT_LOG: Lazy<Arc<Mutex<Vec<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

static IS_SCRIPT_RUNNING: Lazy<Arc<std::sync::atomic::AtomicBool>> =
    Lazy::new(|| Arc::new(std::sync::atomic::AtomicBool::new(false)));

fn build_engine() -> Engine {
    let mut engine = Engine::new();

    // ── Mouse ────────────────────────────────────────────────────────────────

    engine.register_fn("click", |x: i64, y: i64| {
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            e.move_mouse(x as i32, y as i32, Coordinate::Abs).ok();
            e.button(Button::Left, Direction::Click).ok();
        }
    });

    engine.register_fn("right_click", |x: i64, y: i64| {
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            e.move_mouse(x as i32, y as i32, Coordinate::Abs).ok();
            e.button(Button::Right, Direction::Click).ok();
        }
    });

    engine.register_fn("double_click", |x: i64, y: i64| {
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            e.move_mouse(x as i32, y as i32, Coordinate::Abs).ok();
            e.button(Button::Left, Direction::Click).ok();
            thread::sleep(Duration::from_millis(50));
            e.button(Button::Left, Direction::Click).ok();
        }
    });

    engine.register_fn("mouse_move", |x: i64, y: i64| {
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            e.move_mouse(x as i32, y as i32, Coordinate::Abs).ok();
        }
    });

    engine.register_fn("scroll", |amount: i64| {
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            e.scroll(amount as i32, enigo::Axis::Vertical).ok();
        }
    });

    // ── Keyboard ─────────────────────────────────────────────────────────────

    engine.register_fn("type_text", |text: String| {
        let expanded = variables::substitute(&text);
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            for ch in expanded.chars() {
                e.key(Key::Unicode(ch), Direction::Click).ok();
            }
        }
    });

    engine.register_fn("key", |k: String| {
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            if let Some(key) = crate::player::parse_key(&k) {
                e.key(key, Direction::Click).ok();
            }
        }
    });

    engine.register_fn("hotkey", |modifier: String, k: String| {
        if let Ok(mut e) = Enigo::new(&Settings::default()) {
            let mod_key = match modifier.to_lowercase().as_str() {
                "ctrl" | "control" => Some(Key::Control),
                "alt" => Some(Key::Alt),
                "shift" => Some(Key::Shift),
                "win" | "meta" => Some(Key::Meta),
                _ => None,
            };
            if let (Some(mk), Some(key)) = (mod_key, crate::player::parse_key(&k)) {
                e.key(mk, Direction::Press).ok();
                e.key(key, Direction::Click).ok();
                e.key(mk, Direction::Release).ok();
            }
        }
    });

    // ── Timing ───────────────────────────────────────────────────────────────

    engine.register_fn("sleep", |ms: i64| {
        thread::sleep(Duration::from_millis(ms.max(0) as u64));
    });

    // ── Screen / image ────────────────────────────────────────────────────────

    engine.register_fn("get_pixel", |x: i64, y: i64| -> rhai::Array {
        match image_match::get_pixel_color(x as i32, y as i32) {
            Ok(c) => vec![
                rhai::Dynamic::from_int(c[0] as i64),
                rhai::Dynamic::from_int(c[1] as i64),
                rhai::Dynamic::from_int(c[2] as i64),
            ],
            Err(_) => vec![
                rhai::Dynamic::from_int(0i64),
                rhai::Dynamic::from_int(0i64),
                rhai::Dynamic::from_int(0i64),
            ],
        }
    });

    engine.register_fn("wait_pixel", |x: i64, y: i64, r: i64, g: i64, b: i64, tolerance: i64, timeout_ms: i64| -> bool {
        image_match::wait_for_pixel(
            x as i32, y as i32,
            [r as u8, g as u8, b as u8],
            tolerance as u8,
            timeout_ms as u64,
        )
    });

    engine.register_fn("find_image", |template_b64: String, confidence: f64| -> rhai::Array {
        let (found, cx, cy) = image_match::wait_for_image(&template_b64, confidence as f32, None, 0);
        vec![
            rhai::Dynamic::from_bool(found),
            rhai::Dynamic::from_int(cx as i64),
            rhai::Dynamic::from_int(cy as i64),
        ]
    });

    engine.register_fn("wait_image", |template_b64: String, confidence: f64, timeout_ms: i64| -> rhai::Array {
        let (found, cx, cy) = image_match::wait_for_image(&template_b64, confidence as f32, None, timeout_ms as u64);
        vec![
            rhai::Dynamic::from_bool(found),
            rhai::Dynamic::from_int(cx as i64),
            rhai::Dynamic::from_int(cy as i64),
        ]
    });

    engine.register_fn("click_image", |template_b64: String, confidence: f64, timeout_ms: i64| -> bool {
        let (found, cx, cy) = image_match::wait_for_image(&template_b64, confidence as f32, None, timeout_ms as u64);
        if found {
            if let Ok(mut e) = Enigo::new(&Settings::default()) {
                e.move_mouse(cx, cy, Coordinate::Abs).ok();
                e.button(Button::Left, Direction::Click).ok();
            }
        }
        found
    });

    // ── Variables ────────────────────────────────────────────────────────────

    engine.register_fn("get_var", |name: String| -> String {
        variables::preview_variable(format!("{{{}}}", name))
    });

    engine.register_fn("set_var", |name: String, value: String| {
        variables::set_variable(name, value);
    });

    // ── Window ───────────────────────────────────────────────────────────────

    engine.register_fn("get_window_title", || -> String {
        window::get_foreground_window_title()
    });

    engine.register_fn("window_matches", |target: String| -> bool {
        window::active_window_matches(&target)
    });

    // ── Logging ───────────────────────────────────────────────────────────────

    engine.register_fn("log", |msg: String| {
        SCRIPT_LOG.lock().unwrap().push(msg);
    });

    engine.register_fn("print", |msg: String| {
        SCRIPT_LOG.lock().unwrap().push(msg);
    });

    engine
}

pub fn run_script_inline(script: &str) -> Result<(), String> {
    let engine = build_engine();
    let mut scope = Scope::new();
    engine
        .run_with_scope(&mut scope, script)
        .map_err(|e| e.to_string())
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_script(script: String) -> Result<Vec<String>, String> {
    if IS_SCRIPT_RUNNING.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Script already running".into());
    }
    IS_SCRIPT_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);
    SCRIPT_LOG.lock().unwrap().clear();

    let is_running = IS_SCRIPT_RUNNING.clone();
    let _log = SCRIPT_LOG.clone();

    let result = tokio::task::spawn_blocking(move || {
        let res = run_script_inline(&script);
        is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        res
    })
    .await
    .map_err(|e| e.to_string())?;

    let log_lines = SCRIPT_LOG.lock().unwrap().clone();

    match result {
        Ok(()) => Ok(log_lines),
        Err(e) => {
            let mut lines = log_lines;
            lines.push(format!("ERROR: {}", e));
            Err(lines.join("\n"))
        }
    }
}

#[tauri::command]
pub fn get_script_log() -> Vec<String> {
    SCRIPT_LOG.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_script_macro(name: String, script: String) -> Result<(), String> {
    use crate::types::{Macro, MacroEvent, MacroEventData};
    use crate::player::save_macro;
    let m = Macro {
        name,
        events: vec![MacroEvent {
            timestamp_ms: 0,
            event: MacroEventData::RunScript { script },
        }],
        duration_ms: 0,
        tags: vec!["script".into()],
        description: "Rhai script macro".into(),
        relative_window: None,
    };
    save_macro(m).map(|_| ())
}
