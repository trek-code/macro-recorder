use serde::{Deserialize, Serialize};

// ── Macro event types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MacroEventData {
    // Input events (recorded)
    MouseMove { x: f64, y: f64 },
    MousePress { x: f64, y: f64, button: String },
    MouseRelease { x: f64, y: f64, button: String },
    KeyPress { key: String },
    KeyRelease { key: String },
    Scroll { delta_x: i64, delta_y: i64 },

    // Authored steps (added via step editor / script)
    TypeText { text: String },           // type literal text or {variables}
    Sleep { duration_ms: u64 },          // explicit delay

    // Wait-for conditions
    WaitForPixel {
        x: i32,
        y: i32,
        color: [u8; 3],
        tolerance: u8,
        timeout_ms: u64,
    },
    WaitForImage {
        template_b64: String,            // base64-encoded PNG crop
        confidence: f32,                 // 0.0–1.0
        region: Option<[i32; 4]>,        // [x, y, w, h] search area, None = full screen
        timeout_ms: u64,
        click_on_found: bool,
    },

    // Inline Rhai script step
    RunScript { script: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEvent {
    pub timestamp_ms: u64,
    pub event: MacroEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    pub events: Vec<MacroEvent>,
    pub duration_ms: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub relative_window: Option<String>,  // window title; coords stored as offsets from window origin
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainItem {
    pub macro_name: String,
    pub loops: u32,
    pub delay_after_ms: u64,
}

// ── Auto-clicker config ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoClickerConfig {
    pub interval_min_ms: u64,
    pub interval_max_ms: u64,
    pub click_button: String,
    pub double_click: bool,
    pub position_mode: String,  // "cursor" | "fixed" | "bbox"
    pub fixed_x: f64,
    pub fixed_y: f64,
    pub bbox_x1: f64,
    pub bbox_y1: f64,
    pub bbox_x2: f64,
    pub bbox_y2: f64,
    pub humanize: bool,
    pub jitter_px: i32,
    pub stop_after_clicks: u32,
    pub stop_after_seconds: u64,
    pub window_target: String,
}

impl Default for AutoClickerConfig {
    fn default() -> Self {
        Self {
            interval_min_ms: 800,
            interval_max_ms: 1200,
            click_button: "left".into(),
            double_click: false,
            position_mode: "cursor".into(),
            fixed_x: 0.0, fixed_y: 0.0,
            bbox_x1: 0.0, bbox_y1: 0.0,
            bbox_x2: 1920.0, bbox_y2: 1080.0,
            humanize: false,
            jitter_px: 5,
            stop_after_clicks: 0,
            stop_after_seconds: 0,
            window_target: String::new(),
        }
    }
}

// ── Profile ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub playback_speed: f64,
    pub playback_loops: u32,
    pub hotkey_record: String,
    pub hotkey_play: String,
    pub hotkey_autoclicker: String,
    pub autoclicker: AutoClickerConfig,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            playback_speed: 1.0,
            playback_loops: 1,
            hotkey_record: "F9".into(),
            hotkey_play: "F10".into(),
            hotkey_autoclicker: "F11".into(),
            autoclicker: AutoClickerConfig::default(),
        }
    }
}

// ── Scheduler ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub macro_name: String,
    /// Cron expression (e.g. "0 9 * * 1-5") or "once:<unix_ms>"
    pub schedule: String,
    pub speed: f64,
    pub loops: u32,
    pub enabled: bool,
    pub last_run_ms: Option<u64>,
}

// ── License ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LicenseInfo {
    pub tier: String,      // "free" | "pro" | "lifetime"
    pub valid: bool,
    pub features: Vec<String>,
    pub expires_ms: Option<u64>,
    pub cached_at_ms: u64,
}

impl LicenseInfo {
    pub fn free() -> Self {
        Self {
            tier: "free".into(),
            valid: false,
            features: vec![],
            expires_ms: None,
            cached_at_ms: 0,
        }
    }

    pub fn has_feature(&self, f: &str) -> bool {
        self.features.iter().any(|x| x == f)
    }
}
