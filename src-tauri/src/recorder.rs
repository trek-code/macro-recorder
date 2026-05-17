use crate::types::{Macro, MacroEvent, MacroEventData};
use crate::window;
use once_cell::sync::Lazy;
use rdev::{listen, Event, EventType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub static IS_RECORDING: Lazy<Arc<AtomicBool>> =
    Lazy::new(|| Arc::new(AtomicBool::new(false)));

pub static IS_PAUSED: Lazy<Arc<AtomicBool>> =
    Lazy::new(|| Arc::new(AtomicBool::new(false)));

static RECORDED_EVENTS: Lazy<Arc<Mutex<Vec<MacroEvent>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

static RECORD_START_MS: Lazy<Arc<Mutex<u64>>> =
    Lazy::new(|| Arc::new(Mutex::new(0)));

static PAUSE_START_MS: Lazy<Arc<Mutex<u64>>> =
    Lazy::new(|| Arc::new(Mutex::new(0)));

static TOTAL_PAUSED_MS: Lazy<Arc<Mutex<u64>>> =
    Lazy::new(|| Arc::new(Mutex::new(0)));

static LAST_MOUSE: Lazy<Arc<Mutex<(f64, f64)>>> =
    Lazy::new(|| Arc::new(Mutex::new((0.0, 0.0))));

// Window origin for relative recording mode
static WINDOW_ORIGIN: Lazy<Arc<Mutex<Option<(i32, i32)>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn start_recording_impl() -> Result<(), String> {
    if IS_RECORDING.load(Ordering::SeqCst) {
        return Err("Already recording".into());
    }
    {
        RECORDED_EVENTS.lock().unwrap().clear();
        *RECORD_START_MS.lock().unwrap() = now_ms();
        *TOTAL_PAUSED_MS.lock().unwrap() = 0;
        *WINDOW_ORIGIN.lock().unwrap() = None;
    }
    IS_RECORDING.store(true, Ordering::SeqCst);
    IS_PAUSED.store(false, Ordering::SeqCst);

    let is_recording = IS_RECORDING.clone();
    let is_paused    = IS_PAUSED.clone();
    let recorded     = RECORDED_EVENTS.clone();
    let start_ms     = RECORD_START_MS.clone();
    let total_paused = TOTAL_PAUSED_MS.clone();
    let last_mouse   = LAST_MOUSE.clone();
    let win_origin   = WINDOW_ORIGIN.clone();

    thread::spawn(move || {
        listen(move |event: Event| {
            if !is_recording.load(Ordering::SeqCst) || is_paused.load(Ordering::SeqCst) {
                return;
            }
            let t0 = *start_ms.lock().unwrap();
            let paused = *total_paused.lock().unwrap();
            let ts = now_ms().saturating_sub(t0).saturating_sub(paused);

            let maybe_ev: Option<MacroEventData> = match event.event_type {
                EventType::MouseMove { x, y } => {
                    *last_mouse.lock().unwrap() = (x, y);
                    let (rx, ry) = apply_relative(x, y, &win_origin);
                    Some(MacroEventData::MouseMove { x: rx, y: ry })
                }
                EventType::ButtonPress(btn) => {
                    let (mx, my) = *last_mouse.lock().unwrap();
                    let (rx, ry) = apply_relative(mx, my, &win_origin);
                    Some(MacroEventData::MousePress { x: rx, y: ry, button: format!("{:?}", btn) })
                }
                EventType::ButtonRelease(btn) => {
                    let (mx, my) = *last_mouse.lock().unwrap();
                    let (rx, ry) = apply_relative(mx, my, &win_origin);
                    Some(MacroEventData::MouseRelease { x: rx, y: ry, button: format!("{:?}", btn) })
                }
                EventType::KeyPress(key) => Some(MacroEventData::KeyPress { key: format!("{:?}", key) }),
                EventType::KeyRelease(key) => Some(MacroEventData::KeyRelease { key: format!("{:?}", key) }),
                EventType::Wheel { delta_x, delta_y } => Some(MacroEventData::Scroll { delta_x, delta_y }),
            };

            if let Some(ev) = maybe_ev {
                recorded.lock().unwrap().push(MacroEvent { timestamp_ms: ts, event: ev });
            }
        }).ok();
    });

    Ok(())
}

fn apply_relative(x: f64, y: f64, origin: &Arc<Mutex<Option<(i32, i32)>>>) -> (f64, f64) {
    if let Some((ox, oy)) = *origin.lock().unwrap() {
        (x - ox as f64, y - oy as f64)
    } else {
        (x, y)
    }
}

pub fn stop_recording_impl(name: String) -> Result<Macro, String> {
    IS_RECORDING.store(false, Ordering::SeqCst);
    IS_PAUSED.store(false, Ordering::SeqCst);
    let events = RECORDED_EVENTS.lock().unwrap().clone();
    let duration_ms = events.last().map(|e| e.timestamp_ms).unwrap_or(0);
    Ok(Macro {
        name,
        events,
        duration_ms,
        tags: vec![],
        description: String::new(),
        relative_window: WINDOW_ORIGIN.lock().unwrap().map(|_| {
            window::get_foreground_window_title()
        }),
    })
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn start_recording() -> Result<(), String> {
    start_recording_impl()
}

#[tauri::command]
pub fn start_recording_relative() -> Result<(), String> {
    // Capture current foreground window origin before recording
    let title = window::get_foreground_window_title();
    if title.is_empty() {
        return Err("No foreground window found".into());
    }
    let origin = window::get_window_rect(&title).map(|(x, y, _, _)| (x, y));
    *WINDOW_ORIGIN.lock().unwrap() = origin;
    start_recording_impl()
}

#[tauri::command]
pub fn stop_recording(name: String) -> Result<Macro, String> {
    stop_recording_impl(name)
}

#[tauri::command]
pub fn pause_recording() -> Result<(), String> {
    if !IS_RECORDING.load(Ordering::SeqCst) {
        return Err("Not recording".into());
    }
    if !IS_PAUSED.load(Ordering::SeqCst) {
        *PAUSE_START_MS.lock().unwrap() = now_ms();
        IS_PAUSED.store(true, Ordering::SeqCst);
    }
    Ok(())
}

#[tauri::command]
pub fn resume_recording() -> Result<(), String> {
    if IS_PAUSED.load(Ordering::SeqCst) {
        let paused_for = now_ms().saturating_sub(*PAUSE_START_MS.lock().unwrap());
        *TOTAL_PAUSED_MS.lock().unwrap() += paused_for;
        IS_PAUSED.store(false, Ordering::SeqCst);
    }
    Ok(())
}
