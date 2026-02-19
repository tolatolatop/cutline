use std::path::{Path, PathBuf};
use tauri::Manager;

use super::model::ProvidersFile;

pub fn providers_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| format!("Failed to resolve app config dir: {}", e))?;
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    Ok(config_dir.join("providers.json"))
}

pub fn load_providers(path: &Path) -> Result<ProvidersFile, String> {
    if !path.exists() {
        return Ok(ProvidersFile::default());
    }
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read providers.json: {}", e))?;
    serde_json::from_str(&data).map_err(|e| format!("Failed to parse providers.json: {}", e))
}

pub fn save_providers_atomic(path: &Path, file: &ProvidersFile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Failed to serialize providers: {}", e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| format!("Failed to write tmp: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Failed to rename tmp: {}", e))?;
    Ok(())
}
