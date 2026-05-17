pub fn active_window_matches(target: &str) -> bool {
    if target.is_empty() { return true; }
    get_foreground_title().to_lowercase().contains(&target.to_lowercase())
}

/// Returns (x, y, width, height) of the window whose title contains `target`.
pub fn get_window_rect(target: &str) -> Option<(i32, i32, i32, i32)> {
    get_window_rect_impl(target)
}

#[tauri::command]
pub fn get_foreground_window_title() -> String {
    get_foreground_title()
}

#[tauri::command]
pub fn get_window_rect_cmd(title: String) -> Option<[i32; 4]> {
    get_window_rect_impl(&title).map(|(x, y, w, h)| [x, y, w, h])
}

// ── Windows implementation ────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn get_foreground_title() -> String {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    unsafe {
        let hwnd = winapi::um::winuser::GetForegroundWindow();
        if hwnd.is_null() { return String::new(); }
        let mut buf = vec![0u16; 512];
        let len = winapi::um::winuser::GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
        if len <= 0 { return String::new(); }
        OsString::from_wide(&buf[..len as usize]).to_string_lossy().into_owned()
    }
}

#[cfg(target_os = "windows")]
fn get_window_rect_impl(target: &str) -> Option<(i32, i32, i32, i32)> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::winuser::{EnumWindows, GetWindowRect, GetWindowTextW, IsWindowVisible};
    use winapi::shared::windef::{HWND, RECT};
    use winapi::shared::minwindef::{BOOL, LPARAM};

    struct SearchState {
        target: String,
        result: Option<(i32, i32, i32, i32)>,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam as *mut SearchState);
        if IsWindowVisible(hwnd) == 0 { return 1; }
        let mut buf = vec![0u16; 512];
        let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), 512);
        if len <= 0 { return 1; }
        let title = OsString::from_wide(&buf[..len as usize]).to_string_lossy().to_lowercase();
        if title.contains(&state.target.to_lowercase()) {
            let mut rect: RECT = std::mem::zeroed();
            GetWindowRect(hwnd, &mut rect);
            state.result = Some((
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
            ));
            return 0; // stop enumeration
        }
        1
    }

    let mut state = SearchState { target: target.to_string(), result: None };
    unsafe { EnumWindows(Some(enum_callback), &mut state as *mut _ as LPARAM); }
    state.result
}

#[cfg(not(target_os = "windows"))]
fn get_foreground_title() -> String { String::new() }

#[cfg(not(target_os = "windows"))]
fn get_window_rect_impl(_target: &str) -> Option<(i32, i32, i32, i32)> { None }
