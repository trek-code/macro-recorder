mod autoclicker;
mod compression;
mod image_match;
mod license;
mod player;
mod profiles;
mod recorder;
mod scheduler;
mod script;
mod types;
mod variables;
mod window;

use once_cell::sync::OnceCell;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();

fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        win.show().ok();
        win.set_focus().ok();
    }
}

pub fn register_hotkeys(app: &AppHandle, rec: &str, play: &str, ac: &str) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    if !rec.is_empty() {
        let _ = gs.on_shortcut(rec, |_, _, event| {
            if event.state() == ShortcutState::Pressed { hotkey_toggle_record(); }
        });
    }
    if !play.is_empty() {
        let _ = gs.on_shortcut(play, |_, _, event| {
            if event.state() == ShortcutState::Pressed { hotkey_toggle_play(); }
        });
    }
    if !ac.is_empty() {
        let _ = gs.on_shortcut(ac, |_, _, event| {
            if event.state() == ShortcutState::Pressed { hotkey_toggle_autoclicker(); }
        });
    }
}

fn hotkey_toggle_record() {
    use std::sync::atomic::Ordering;
    if recorder::IS_RECORDING.load(Ordering::SeqCst) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if let Ok(m) = recorder::stop_recording_impl(format!("Hotkey_{}", ts)) {
            let _ = player::save_macro(m);
        }
    } else {
        let _ = recorder::start_recording_impl();
    }
}

fn hotkey_toggle_play() {
    use std::sync::atomic::Ordering;
    if player::IS_PLAYING.load(Ordering::SeqCst) {
        player::stop_playback();
    }
}

fn hotkey_toggle_autoclicker() {
    use std::sync::atomic::Ordering;
    if autoclicker::IS_AUTOCLICKING.load(Ordering::SeqCst) {
        autoclicker::stop_autoclicker();
    } else {
        let config = profiles::ACTIVE_PROFILE.lock().unwrap().autoclicker.clone();
        autoclicker::start_autoclicker_impl(config);
    }
}

#[tauri::command]
fn get_status() -> serde_json::Value {
    use std::sync::atomic::Ordering;
    serde_json::json!({
        "recording":    recorder::IS_RECORDING.load(Ordering::SeqCst),
        "paused":       recorder::IS_PAUSED.load(Ordering::SeqCst),
        "playing":      player::IS_PLAYING.load(Ordering::SeqCst),
        "autoclicking": autoclicker::IS_AUTOCLICKING.load(Ordering::SeqCst),
        "clicks":       autoclicker::CLICK_COUNT.load(Ordering::SeqCst),
    })
}

#[tauri::command]
fn update_hotkeys(rec: String, play: String, ac: String) {
    if let Some(app) = APP_HANDLE.get() {
        register_hotkeys(app, &rec, &play, &ac);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            APP_HANDLE.set(app.handle().clone()).ok();

            // Tray
            let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Macro Recorder v0.3")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                        show_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // Hotkeys
            let profile = profiles::ACTIVE_PROFILE.lock().unwrap().clone();
            register_hotkeys(app.handle(), &profile.hotkey_record, &profile.hotkey_play, &profile.hotkey_autoclicker);

            // Scheduler
            scheduler::start_scheduler();

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().ok();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            update_hotkeys,
            // Recorder
            recorder::start_recording,
            recorder::start_recording_relative,
            recorder::stop_recording,
            recorder::pause_recording,
            recorder::resume_recording,
            // Player
            player::play_macro,
            player::run_chain,
            player::stop_playback,
            player::save_macro,
            player::list_macros,
            player::delete_macro,
            player::update_macro_events,
            player::update_macro_meta,
            player::import_macro,
            player::export_macro,
            // Compression
            compression::compress_macro,
            // Auto-clicker
            autoclicker::start_autoclicker,
            autoclicker::stop_autoclicker,
            autoclicker::get_autoclicker_status,
            // Profiles
            profiles::list_profiles,
            profiles::save_profile,
            profiles::delete_profile,
            profiles::apply_profile,
            profiles::get_active_profile,
            // Window
            window::get_foreground_window_title,
            window::get_window_rect_cmd,
            // Image match
            image_match::capture_screenshot_b64,
            image_match::capture_region_b64,
            image_match::get_pixel_color,
            image_match::find_image_on_screen,
            // Variables
            variables::set_variable,
            variables::delete_variable,
            variables::get_variables,
            variables::reset_counter,
            variables::preview_variable,
            // Scripts
            script::run_script,
            script::get_script_log,
            script::save_script_macro,
            // Scheduler
            scheduler::list_scheduled_tasks,
            scheduler::add_scheduled_task,
            scheduler::toggle_scheduled_task,
            scheduler::delete_scheduled_task,
            scheduler::run_task_now,
            // License
            license::get_license_info,
            license::activate_license,
            license::refresh_license,
            license::get_machine_id_cmd,
            license::has_feature,
            license::deactivate_license,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
