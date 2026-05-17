use crate::types::Profile;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub static ACTIVE_PROFILE: Lazy<Mutex<Profile>> =
    Lazy::new(|| Mutex::new(Profile::default()));

fn profiles_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("MacroRecorder").join("profiles");
    fs::create_dir_all(&dir).ok();
    dir
}

fn safe_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_profiles() -> Result<Vec<Profile>, String> {
    let dir = profiles_dir();
    let mut profiles = vec![Profile::default()];
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(p) = serde_json::from_str::<Profile>(&content) {
                        if p.name != "Default" {
                            profiles.push(p);
                        }
                    }
                }
            }
        }
    }
    Ok(profiles)
}

#[tauri::command]
pub fn save_profile(profile: Profile) -> Result<(), String> {
    let dir = profiles_dir();
    let path = dir.join(format!("{}.json", safe_name(&profile.name)));
    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    if name == "Default" {
        return Err("Cannot delete the Default profile".into());
    }
    let dir = profiles_dir();
    let path = dir.join(format!("{}.json", safe_name(&name)));
    fs::remove_file(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn apply_profile(name: String) -> Result<Profile, String> {
    let profiles = list_profiles()?;
    let profile = profiles
        .into_iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("Profile '{}' not found", name))?;

    *ACTIVE_PROFILE.lock().unwrap() = profile.clone();
    Ok(profile)
}

#[tauri::command]
pub fn get_active_profile() -> Profile {
    ACTIVE_PROFILE.lock().unwrap().clone()
}
