use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, GenericImageView};
use once_cell::sync::Lazy;
use std::io::Cursor;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use xcap::Monitor;

// Cache last screenshot to avoid re-capturing for multiple checks in one macro step
static LAST_SCREENSHOT: Lazy<Mutex<Option<DynamicImage>>> = Lazy::new(|| Mutex::new(None));

// ── Screenshot helpers ────────────────────────────────────────────────────────

pub fn capture_screen() -> Result<DynamicImage, String> {
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    let primary = monitors.into_iter().next().ok_or("No monitor found")?;
    let img = primary.capture_image().map_err(|e| e.to_string())?;
    // xcap returns RgbaImage
    let dyn_img = DynamicImage::ImageRgba8(img);
    Ok(dyn_img)
}

fn fresh_screenshot() -> Result<DynamicImage, String> {
    let img = capture_screen()?;
    *LAST_SCREENSHOT.lock().unwrap() = Some(img.clone());
    Ok(img)
}

// ── Pixel color check ─────────────────────────────────────────────────────────

pub fn pixel_matches(img: &DynamicImage, x: i32, y: i32, target: [u8; 3], tolerance: u8) -> bool {
    if x < 0 || y < 0 { return false; }
    let (w, h) = img.dimensions();
    if x as u32 >= w || y as u32 >= h { return false; }
    let px = img.get_pixel(x as u32, y as u32);
    let diff = |a: u8, b: u8| -> u8 { a.abs_diff(b) };
    diff(px[0], target[0]) <= tolerance
        && diff(px[1], target[1]) <= tolerance
        && diff(px[2], target[2]) <= tolerance
}

/// Wait until pixel at (x,y) matches color within tolerance.
/// Returns true if matched before timeout.
pub fn wait_for_pixel(
    x: i32,
    y: i32,
    color: [u8; 3],
    tolerance: u8,
    timeout_ms: u64,
) -> bool {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Ok(img) = fresh_screenshot() {
            if pixel_matches(&img, x, y, color, tolerance) {
                return true;
            }
        }
        if Instant::now() >= deadline { return false; }
        std::thread::sleep(Duration::from_millis(100));
    }
}

// ── Template matching (SAD) ───────────────────────────────────────────────────

/// Returns (confidence 0.0–1.0, match_x, match_y) of best template match in `haystack`.
/// Optionally restricted to `region` [x, y, w, h].
pub fn find_template(
    haystack: &DynamicImage,
    template: &DynamicImage,
    region: Option<[i32; 4]>,
) -> (f32, i32, i32) {
    let (hw, hh) = haystack.dimensions();
    let (tw, th) = template.dimensions();

    if tw >= hw || th >= hh { return (0.0, 0, 0); }

    // Restrict search area
    let (sx, sy, sw, sh) = if let Some([rx, ry, rw, rh]) = region {
        let sx = rx.max(0) as u32;
        let sy = ry.max(0) as u32;
        let sw = (rw as u32).min(hw.saturating_sub(sx));
        let sh = (rh as u32).min(hh.saturating_sub(sy));
        (sx, sy, sw, sh)
    } else {
        (0, 0, hw, hh)
    };

    let search_w = sw.saturating_sub(tw) + 1;
    let search_h = sh.saturating_sub(th) + 1;

    // Precompute template pixel values
    let t_pixels: Vec<[u8; 3]> = (0..th)
        .flat_map(|y| (0..tw).map(move |x| {
            let p = template.get_pixel(x, y);
            [p[0], p[1], p[2]]
        }))
        .collect();

    let max_sad = (tw * th) as f64 * 255.0 * 3.0;
    let mut best_conf: f32 = 0.0;
    let mut best_x = 0i32;
    let mut best_y = 0i32;

    for oy in 0..search_h {
        for ox in 0..search_w {
            let abs_x = sx + ox;
            let abs_y = sy + oy;
            let mut sad: f64 = 0.0;

            'outer: for ty in 0..th {
                for tx in 0..tw {
                    let hp = haystack.get_pixel(abs_x + tx, abs_y + ty);
                    let tp = &t_pixels[(ty * tw + tx) as usize];
                    sad += (hp[0] as i32 - tp[0] as i32).unsigned_abs() as f64
                        + (hp[1] as i32 - tp[1] as i32).unsigned_abs() as f64
                        + (hp[2] as i32 - tp[2] as i32).unsigned_abs() as f64;
                    // Early exit: if SAD already worse than best, skip
                    if sad > max_sad * (1.0 - best_conf as f64) {
                        break 'outer;
                    }
                }
            }

            let conf = 1.0 - (sad / max_sad) as f32;
            if conf > best_conf {
                best_conf = conf;
                best_x = (abs_x + tw / 2) as i32;
                best_y = (abs_y + th / 2) as i32;
            }
        }
    }

    (best_conf, best_x, best_y)
}

/// Wait for template to appear on screen. Returns (found, center_x, center_y).
pub fn wait_for_image(
    template_b64: &str,
    confidence: f32,
    region: Option<[i32; 4]>,
    timeout_ms: u64,
) -> (bool, i32, i32) {
    let template = match decode_template(template_b64) {
        Ok(t) => t,
        Err(_) => return (false, 0, 0),
    };

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let Ok(screen) = fresh_screenshot() {
            let (conf, cx, cy) = find_template(&screen, &template, region);
            if conf >= confidence {
                return (true, cx, cy);
            }
        }
        if Instant::now() >= deadline { return (false, 0, 0); }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn decode_template(b64: &str) -> Result<DynamicImage, String> {
    let bytes = STANDARD.decode(b64).map_err(|e| e.to_string())?;
    image::load_from_memory(&bytes).map_err(|e| e.to_string())
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn capture_screenshot_b64() -> Result<String, String> {
    let img = capture_screen()?;
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(&buf))
}

/// Capture a region of the screen and return base64 PNG (used to create templates).
#[tauri::command]
pub fn capture_region_b64(x: i32, y: i32, w: u32, h: u32) -> Result<String, String> {
    let img = capture_screen()?;
    let cropped = img.crop_imm(x.max(0) as u32, y.max(0) as u32, w, h);
    let mut buf = Vec::new();
    cropped
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(STANDARD.encode(&buf))
}

#[tauri::command]
pub fn get_pixel_color(x: i32, y: i32) -> Result<[u8; 3], String> {
    let img = fresh_screenshot()?;
    if x < 0 || y < 0 { return Err("Negative coords".into()); }
    let (w, h) = img.dimensions();
    if x as u32 >= w || y as u32 >= h { return Err("Out of bounds".into()); }
    let p = img.get_pixel(x as u32, y as u32);
    Ok([p[0], p[1], p[2]])
}

#[tauri::command]
pub fn find_image_on_screen(
    template_b64: String,
    confidence: f32,
    region: Option<[i32; 4]>,
) -> Result<Option<[i32; 2]>, String> {
    let template = decode_template(&template_b64)?;
    let screen = fresh_screenshot()?;
    let (conf, cx, cy) = find_template(&screen, &template, region);
    if conf >= confidence {
        Ok(Some([cx, cy]))
    } else {
        Ok(None)
    }
}
