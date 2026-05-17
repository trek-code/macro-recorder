use chrono::Local;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static COUNTERS: Lazy<Mutex<HashMap<String, u64>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static CUSTOM_VARS: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Substitute all `{variable}` tokens in `text`.
pub fn substitute(text: &str) -> String {
    let mut result = text.to_string();
    result = result.replace("{date}", &Local::now().format("%Y-%m-%d").to_string());
    result = result.replace("{time}", &Local::now().format("%H:%M:%S").to_string());
    result = result.replace("{datetime}", &Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

    // {counter} — default counter
    if result.contains("{counter}") {
        let val = bump_counter("__default__");
        result = result.replace("{counter}", &val.to_string());
    }

    // {counter:name} — named counters; scan manually
    result = replace_named_counters(result);

    // {clipboard}
    if result.contains("{clipboard}") {
        result = result.replace("{clipboard}", &get_clipboard());
    }

    // Custom variables
    for (k, v) in CUSTOM_VARS.lock().unwrap().iter() {
        result = result.replace(&format!("{{{}}}", k), v);
    }

    result
}

fn bump_counter(name: &str) -> u64 {
    let mut counters = COUNTERS.lock().unwrap();
    let n = counters.entry(name.to_string()).or_insert(0);
    *n += 1;
    *n
}

fn replace_named_counters(mut s: String) -> String {
    loop {
        if let Some(start) = s.find("{counter:") {
            if let Some(end) = s[start..].find('}') {
                let token = &s[start..start + end + 1].to_string();
                let name = &token[9..token.len() - 1]; // strip {counter: and }
                let val = bump_counter(name);
                s = s.replacen(token, &val.to_string(), 1);
                continue;
            }
        }
        break;
    }
    s
}

fn get_clipboard() -> String {
    #[cfg(target_os = "windows")]
    { read_clipboard_windows().unwrap_or_default() }
    #[cfg(not(target_os = "windows"))]
    { String::new() }
}

#[cfg(target_os = "windows")]
fn read_clipboard_windows() -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    unsafe {
        if winapi::um::winuser::OpenClipboard(std::ptr::null_mut()) == 0 { return None; }
        let h = winapi::um::winuser::GetClipboardData(winapi::um::winuser::CF_UNICODETEXT as u32);
        if h.is_null() { winapi::um::winuser::CloseClipboard(); return None; }
        let ptr = winapi::um::winbase::GlobalLock(h) as *const u16;
        if ptr.is_null() { winapi::um::winuser::CloseClipboard(); return None; }
        let mut len = 0usize;
        while *ptr.add(len) != 0 { len += 1; }
        let slice = std::slice::from_raw_parts(ptr, len);
        let s = OsString::from_wide(slice).to_string_lossy().into_owned();
        winapi::um::winbase::GlobalUnlock(h);
        winapi::um::winuser::CloseClipboard();
        Some(s)
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn set_variable(name: String, value: String) {
    CUSTOM_VARS.lock().unwrap().insert(name, value);
}

#[tauri::command]
pub fn delete_variable(name: String) {
    CUSTOM_VARS.lock().unwrap().remove(&name);
}

#[tauri::command]
pub fn get_variables() -> HashMap<String, String> {
    let mut vars = HashMap::new();
    vars.insert("date".into(), Local::now().format("%Y-%m-%d").to_string());
    vars.insert("time".into(), Local::now().format("%H:%M:%S").to_string());
    vars.insert("datetime".into(), Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
    vars.insert("clipboard".into(), get_clipboard());
    for (k, v) in CUSTOM_VARS.lock().unwrap().iter() {
        vars.insert(k.clone(), v.clone());
    }
    vars
}

#[tauri::command]
pub fn reset_counter(name: Option<String>) {
    let key = name.unwrap_or_else(|| "__default__".into());
    COUNTERS.lock().unwrap().insert(key, 0);
}

#[tauri::command]
pub fn preview_variable(text: String) -> String {
    substitute(&text)
}
