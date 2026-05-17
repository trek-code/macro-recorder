use crate::types::LicenseInfo;
use dirs;
use hmac::{Hmac, Mac};
use mac_address::get_mac_address;
use once_cell::sync::Lazy;
use sha2::Sha256;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

static CACHED_LICENSE: Lazy<Mutex<LicenseInfo>> =
    Lazy::new(|| Mutex::new(LicenseInfo::free()));

const CACHE_TTL_MS: u64 = 24 * 60 * 60 * 1000; // 24 hours

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn license_cache_path() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("MacroRecorder").join("license.json")
}

pub fn get_machine_id() -> String {
    if let Ok(Some(mac)) = get_mac_address() {
        return hex::encode(mac.bytes());
    }
    // Fallback: hash the username + hostname
    let user = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".into());
    let host = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".into());
    let input = format!("{}-{}", user, host);
    let mut mac_hmac = HmacSha256::new_from_slice(b"machine-id-salt").unwrap();
    mac_hmac.update(input.as_bytes());
    hex::encode(mac_hmac.finalize().into_bytes())
}

fn load_cached() -> Option<LicenseInfo> {
    let content = fs::read_to_string(license_cache_path()).ok()?;
    let info: LicenseInfo = serde_json::from_str(&content).ok()?;
    if now_ms().saturating_sub(info.cached_at_ms) < CACHE_TTL_MS {
        Some(info)
    } else {
        None
    }
}

fn save_cache(info: &LicenseInfo) {
    if let Ok(json) = serde_json::to_string_pretty(info) {
        if let Some(parent) = license_cache_path().parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(license_cache_path(), json).ok();
    }
}

/// Validate key against Supabase edge function.
/// POST https://<project>.supabase.co/functions/v1/validate-license
/// Body: { key, machine_id }
/// Response: LicenseInfo JSON
async fn validate_online(key: &str, supabase_url: &str, anon_key: &str) -> Result<LicenseInfo, String> {
    let machine_id = get_machine_id();
    let client = reqwest::Client::new();
    let url = format!("{}/functions/v1/validate-license", supabase_url);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", anon_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "key": key, "machine_id": machine_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Validation failed ({}): {}", status, body));
    }

    let mut info: LicenseInfo = response.json().await.map_err(|e| e.to_string())?;
    info.cached_at_ms = now_ms();
    Ok(info)
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_license_info() -> LicenseInfo {
    // Return cached if fresh
    if let Some(cached) = load_cached() {
        *CACHED_LICENSE.lock().unwrap() = cached.clone();
        return cached;
    }
    CACHED_LICENSE.lock().unwrap().clone()
}

#[tauri::command]
pub async fn activate_license(
    key: String,
    supabase_url: String,
    anon_key: String,
) -> Result<LicenseInfo, String> {
    let info = validate_online(&key, &supabase_url, &anon_key).await?;
    save_cache(&info);
    *CACHED_LICENSE.lock().unwrap() = info.clone();
    Ok(info)
}

#[tauri::command]
pub async fn refresh_license(
    key: String,
    supabase_url: String,
    anon_key: String,
) -> Result<LicenseInfo, String> {
    activate_license(key, supabase_url, anon_key).await
}

#[tauri::command]
pub fn get_machine_id_cmd() -> String {
    get_machine_id()
}

#[tauri::command]
pub fn has_feature(feature: String) -> bool {
    get_license_info().has_feature(&feature)
}

#[tauri::command]
pub fn deactivate_license() {
    fs::remove_file(license_cache_path()).ok();
    *CACHED_LICENSE.lock().unwrap() = LicenseInfo::free();
}
