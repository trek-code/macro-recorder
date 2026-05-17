use crate::image_match;
use crate::script;
use crate::types::{ChainItem, Macro, MacroEvent, MacroEventData};
use crate::variables;
use crate::window;
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub static IS_PLAYING: Lazy<Arc<AtomicBool>> =
    Lazy::new(|| Arc::new(AtomicBool::new(false)));

fn macros_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("MacroRecorder").join("macros");
    fs::create_dir_all(&dir).ok();
    dir
}

fn load_macro_by_name(name: &str) -> Option<Macro> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = macros_dir().join(format!("{}.json", safe));
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Resolve relative coordinates to absolute if macro has a relative_window set.
fn resolve_coords(x: f64, y: f64, window_title: &Option<String>) -> (f64, f64) {
    if let Some(title) = window_title {
        if !title.is_empty() {
            if let Some((wx, wy, _, _)) = window::get_window_rect(title) {
                return (x + wx as f64, y + wy as f64);
            }
        }
    }
    (x, y)
}

fn play_macro_inner(
    enigo: &mut Enigo,
    macro_data: &Macro,
    speed: f64,
    loops: u32,
    is_playing: &Arc<AtomicBool>,
    window_target: &str,
) {
    let effective_speed = speed.max(0.1);

    for _ in 0..loops {
        if !is_playing.load(Ordering::SeqCst) { break; }
        if !window_target.is_empty() && !window::active_window_matches(window_target) {
            thread::sleep(Duration::from_millis(500));
            continue;
        }

        let mut prev_ts: u64 = 0;
        for ev in &macro_data.events {
            if !is_playing.load(Ordering::SeqCst) { break; }

            // Timing delay
            let delay_ms = ev.timestamp_ms.saturating_sub(prev_ts);
            let adjusted = (delay_ms as f64 / effective_speed) as u64;
            if adjusted > 0 {
                thread::sleep(Duration::from_millis(adjusted));
            }
            prev_ts = ev.timestamp_ms;

            dispatch_event(enigo, &ev.event, &macro_data.relative_window);
        }
    }
}

fn dispatch_event(enigo: &mut Enigo, ev: &MacroEventData, relative_window: &Option<String>) {
    match ev {
        MacroEventData::MouseMove { x, y } => {
            let (ax, ay) = resolve_coords(*x, *y, relative_window);
            enigo.move_mouse(ax as i32, ay as i32, Coordinate::Abs).ok();
        }
        MacroEventData::MousePress { x, y, button } => {
            let (ax, ay) = resolve_coords(*x, *y, relative_window);
            enigo.move_mouse(ax as i32, ay as i32, Coordinate::Abs).ok();
            enigo.button(parse_button(button), Direction::Press).ok();
        }
        MacroEventData::MouseRelease { x, y, button } => {
            let (ax, ay) = resolve_coords(*x, *y, relative_window);
            enigo.move_mouse(ax as i32, ay as i32, Coordinate::Abs).ok();
            enigo.button(parse_button(button), Direction::Release).ok();
        }
        MacroEventData::KeyPress { key } => {
            if let Some(k) = parse_key(key) { enigo.key(k, Direction::Press).ok(); }
        }
        MacroEventData::KeyRelease { key } => {
            if let Some(k) = parse_key(key) { enigo.key(k, Direction::Release).ok(); }
        }
        MacroEventData::Scroll { delta_x, delta_y } => {
            enigo.scroll(*delta_x as i32, enigo::Axis::Horizontal).ok();
            enigo.scroll(*delta_y as i32, enigo::Axis::Vertical).ok();
        }

        // ── New event types ──────────────────────────────────────────────────

        MacroEventData::TypeText { text } => {
            let expanded = variables::substitute(text);
            for ch in expanded.chars() {
                enigo.key(Key::Unicode(ch), Direction::Click).ok();
            }
        }

        MacroEventData::Sleep { duration_ms } => {
            thread::sleep(Duration::from_millis(*duration_ms));
        }

        MacroEventData::WaitForPixel { x, y, color, tolerance, timeout_ms } => {
            image_match::wait_for_pixel(*x, *y, *color, *tolerance, *timeout_ms);
        }

        MacroEventData::WaitForImage { template_b64, confidence, region, timeout_ms, click_on_found } => {
            let (found, cx, cy) = image_match::wait_for_image(
                template_b64,
                *confidence,
                *region,
                *timeout_ms,
            );
            if found && *click_on_found {
                enigo.move_mouse(cx, cy, Coordinate::Abs).ok();
                enigo.button(Button::Left, Direction::Click).ok();
            }
        }

        MacroEventData::RunScript { script } => {
            script::run_script_inline(script).ok();
        }
    }
}

/// Blocking version used by scheduler
pub fn play_macro_blocking(macro_data: Macro, speed: f64, loops: u32, window_target: String) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    let flag = Arc::new(AtomicBool::new(true));
    play_macro_inner(&mut enigo, &macro_data, speed, loops, &flag, &window_target);
    Ok(())
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn play_macro(macro_data: Macro, speed: f64, loops: u32, window_target: String) -> Result<(), String> {
    if IS_PLAYING.load(Ordering::SeqCst) { return Err("Already playing".into()); }
    IS_PLAYING.store(true, Ordering::SeqCst);
    let is_playing = IS_PLAYING.clone();

    tokio::task::spawn_blocking(move || {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        play_macro_inner(&mut enigo, &macro_data, speed, loops, &is_playing, &window_target);
        is_playing.store(false, Ordering::SeqCst);
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_chain(chain: Vec<ChainItem>, speed: f64, window_target: String) -> Result<(), String> {
    if IS_PLAYING.load(Ordering::SeqCst) { return Err("Already playing".into()); }
    IS_PLAYING.store(true, Ordering::SeqCst);
    let is_playing = IS_PLAYING.clone();

    tokio::task::spawn_blocking(move || {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        for item in &chain {
            if !is_playing.load(Ordering::SeqCst) { break; }
            if let Some(m) = load_macro_by_name(&item.macro_name) {
                play_macro_inner(&mut enigo, &m, speed, item.loops, &is_playing, &window_target);
                if item.delay_after_ms > 0 {
                    thread::sleep(Duration::from_millis(item.delay_after_ms));
                }
            }
        }
        is_playing.store(false, Ordering::SeqCst);
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn stop_playback() {
    IS_PLAYING.store(false, Ordering::SeqCst);
}

#[tauri::command]
pub fn save_macro(macro_data: Macro) -> Result<String, String> {
    let dir = macros_dir();
    let safe_name: String = macro_data
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = dir.join(format!("{}.json", safe_name));
    let json = serde_json::to_string_pretty(&macro_data).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_macros() -> Result<Vec<Macro>, String> {
    let dir = macros_dir();
    let mut macros = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(m) = serde_json::from_str::<Macro>(&content) {
                        macros.push(m);
                    }
                }
            }
        }
    }
    Ok(macros)
}

#[tauri::command]
pub fn delete_macro(name: String) -> Result<(), String> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    fs::remove_file(macros_dir().join(format!("{}.json", safe))).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_macro_events(name: String, events: Vec<MacroEvent>) -> Result<(), String> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = macros_dir().join(format!("{}.json", safe));
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut m: Macro = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    m.duration_ms = events.last().map(|e| e.timestamp_ms).unwrap_or(0);
    m.events = events;
    let json = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_macro_meta(name: String, tags: Vec<String>, description: String) -> Result<(), String> {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = macros_dir().join(format!("{}.json", safe));
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut m: Macro = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    m.tags = tags;
    m.description = description;
    let json = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_macro(path: String) -> Result<Macro, String> {
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let m: Macro = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    save_macro(m.clone())?;
    Ok(m)
}

#[tauri::command]
pub fn export_macro(name: String, path: String) -> Result<(), String> {
    let m = load_macro_by_name(&name).ok_or_else(|| format!("Macro '{}' not found", name))?;
    let json = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

// ── Key / button helpers ──────────────────────────────────────────────────────

pub fn parse_button(s: &str) -> Button {
    match s {
        "Right" => Button::Right,
        "Middle" => Button::Middle,
        _ => Button::Left,
    }
}

pub fn parse_key(s: &str) -> Option<Key> {
    Some(match s {
        "Return" | "KpReturn" => Key::Return,
        "Space" => Key::Space,
        "BackSpace" => Key::Backspace,
        "Tab" => Key::Tab,
        "Escape" => Key::Escape,
        "Delete" => Key::Delete,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "UpArrow" => Key::UpArrow,
        "DownArrow" => Key::DownArrow,
        "LeftArrow" => Key::LeftArrow,
        "RightArrow" => Key::RightArrow,
        "ShiftLeft" | "ShiftRight" => Key::Shift,
        "ControlLeft" | "ControlRight" => Key::Control,
        "Alt" | "AltGr" => Key::Alt,
        "MetaLeft" | "MetaRight" => Key::Meta,
        "CapsLock" => Key::CapsLock,
        "F1"  => Key::F1,  "F2"  => Key::F2,  "F3"  => Key::F3,
        "F4"  => Key::F4,  "F5"  => Key::F5,  "F6"  => Key::F6,
        "F7"  => Key::F7,  "F8"  => Key::F8,  "F9"  => Key::F9,
        "F10" => Key::F10, "F11" => Key::F11, "F12" => Key::F12,
        other => {
            if other.starts_with("Key") && other.len() == 4 {
                Key::Unicode(other.chars().nth(3)?.to_ascii_lowercase())
            } else if other.starts_with("Num") && other.len() == 4 {
                Key::Unicode(other.chars().nth(3)?)
            } else {
                return None;
            }
        }
    })
}
