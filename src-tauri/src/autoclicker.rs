use crate::types::AutoClickerConfig;
use crate::window;
use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use once_cell::sync::Lazy;
use rand::Rng;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub static IS_AUTOCLICKING: Lazy<Arc<AtomicBool>> =
    Lazy::new(|| Arc::new(AtomicBool::new(false)));

pub static CLICK_COUNT: Lazy<Arc<AtomicU64>> =
    Lazy::new(|| Arc::new(AtomicU64::new(0)));

static ACTIVE_CONFIG: Lazy<Arc<Mutex<AutoClickerConfig>>> =
    Lazy::new(|| Arc::new(Mutex::new(AutoClickerConfig::default())));

pub fn start_autoclicker_impl(config: AutoClickerConfig) {
    if IS_AUTOCLICKING.load(Ordering::SeqCst) {
        return;
    }
    *ACTIVE_CONFIG.lock().unwrap() = config.clone();
    CLICK_COUNT.store(0, Ordering::SeqCst);
    IS_AUTOCLICKING.store(true, Ordering::SeqCst);

    let is_clicking = IS_AUTOCLICKING.clone();
    let click_count = CLICK_COUNT.clone();

    thread::spawn(move || {
        let mut enigo = match Enigo::new(&Settings::default()) {
            Ok(e) => e,
            Err(_) => {
                is_clicking.store(false, Ordering::SeqCst);
                return;
            }
        };
        let mut rng = rand::thread_rng();
        let start_time = Instant::now();

        while is_clicking.load(Ordering::SeqCst) {
            // Check stop conditions
            let clicks = click_count.load(Ordering::SeqCst);
            if config.stop_after_clicks > 0 && clicks >= config.stop_after_clicks as u64 {
                break;
            }
            if config.stop_after_seconds > 0
                && start_time.elapsed().as_secs() >= config.stop_after_seconds
            {
                break;
            }

            // Check window target
            if !config.window_target.is_empty()
                && !window::active_window_matches(&config.window_target)
            {
                thread::sleep(Duration::from_millis(100));
                continue;
            }

            // Determine click position
            let (cx, cy) = match config.position_mode.as_str() {
                "fixed" => {
                    let jx = if config.humanize {
                        rng.gen_range(-config.jitter_px..=config.jitter_px)
                    } else {
                        0
                    };
                    let jy = if config.humanize {
                        rng.gen_range(-config.jitter_px..=config.jitter_px)
                    } else {
                        0
                    };
                    (config.fixed_x as i32 + jx, config.fixed_y as i32 + jy)
                }
                "bbox" => {
                    let x = rng.gen_range(config.bbox_x1 as i32..=config.bbox_x2 as i32);
                    let y = rng.gen_range(config.bbox_y1 as i32..=config.bbox_y2 as i32);
                    (x, y)
                }
                _ => {
                    // "cursor" — use current position, optionally jitter
                    if config.humanize && config.jitter_px > 0 {
                        if let Ok((mx, my)) = enigo.location() {
                            let jx = rng.gen_range(-config.jitter_px..=config.jitter_px);
                            let jy = rng.gen_range(-config.jitter_px..=config.jitter_px);
                            (mx + jx, my + jy)
                        } else {
                            continue;
                        }
                    } else {
                        // don't move — click wherever cursor is
                        if let Ok(pos) = enigo.location() {
                            pos
                        } else {
                            continue;
                        }
                    }
                }
            };

            let btn = match config.click_button.as_str() {
                "right" => Button::Right,
                "middle" => Button::Middle,
                _ => Button::Left,
            };

            if config.position_mode != "cursor" || config.humanize {
                enigo.move_mouse(cx, cy, Coordinate::Abs).ok();
            }

            if config.double_click {
                enigo.button(btn, Direction::Click).ok();
                let dc_delay = if config.humanize {
                    rng.gen_range(40u64..=80)
                } else {
                    50
                };
                thread::sleep(Duration::from_millis(dc_delay));
                enigo.button(btn, Direction::Click).ok();
            } else {
                let hold_ms = if config.humanize {
                    rng.gen_range(40u64..=120)
                } else {
                    0
                };
                enigo.button(btn, Direction::Press).ok();
                if hold_ms > 0 {
                    thread::sleep(Duration::from_millis(hold_ms));
                }
                enigo.button(btn, Direction::Release).ok();
            }

            click_count.fetch_add(1, Ordering::SeqCst);

            // Wait for next click interval
            let base_interval = rng.gen_range(config.interval_min_ms..=config.interval_max_ms);
            let interval = if config.humanize {
                let variation = (base_interval as f64 * 0.1) as u64;
                let delta: i64 = rng.gen_range(-(variation as i64)..=(variation as i64));
                (base_interval as i64 + delta).max(10) as u64
            } else {
                base_interval
            };
            thread::sleep(Duration::from_millis(interval));
        }

        is_clicking.store(false, Ordering::SeqCst);
    });
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn start_autoclicker(config: AutoClickerConfig) -> Result<(), String> {
    if IS_AUTOCLICKING.load(Ordering::SeqCst) {
        return Err("Already running".into());
    }
    start_autoclicker_impl(config);
    Ok(())
}

#[tauri::command]
pub fn stop_autoclicker() {
    IS_AUTOCLICKING.store(false, Ordering::SeqCst);
}

#[tauri::command]
pub fn get_autoclicker_status() -> serde_json::Value {
    serde_json::json!({
        "running": IS_AUTOCLICKING.load(Ordering::SeqCst),
        "clicks": CLICK_COUNT.load(Ordering::SeqCst),
    })
}
