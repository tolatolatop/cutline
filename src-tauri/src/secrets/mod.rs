use keyring::Entry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

const SERVICE_NAME: &str = "cutline";
const SECRETS_FILE: &str = "secrets.json";

static SECRETS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Must be called once during app setup to enable file-based fallback.
pub fn init(config_dir: PathBuf) {
    let _ = SECRETS_DIR.set(config_dir);
}

fn secrets_file_path() -> Option<PathBuf> {
    SECRETS_DIR.get().map(|d| d.join(SECRETS_FILE))
}

fn load_file_store() -> HashMap<String, String> {
    secrets_file_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_file_store(store: &HashMap<String, String>) -> Result<(), String> {
    let path = secrets_file_path().ok_or("secrets dir not initialized")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create secrets dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| format!("failed to serialize secrets: {}", e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).map_err(|e| format!("failed to write secrets: {}", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("failed to rename secrets: {}", e))?;
    Ok(())
}

fn entry(credential_ref: &str) -> Option<Entry> {
    Entry::new(SERVICE_NAME, credential_ref).ok()
}

pub fn set_secret(credential_ref: &str, secret: &str) -> Result<(), String> {
    let keyring_ok = entry(credential_ref)
        .and_then(|e| e.set_password(secret).ok())
        .is_some();

    if !keyring_ok {
        log::warn!("Keyring unavailable, using file-based secret storage");
    }

    let mut store = load_file_store();
    store.insert(credential_ref.to_string(), secret.to_string());
    save_file_store(&store)?;

    Ok(())
}

pub fn get_secret(credential_ref: &str) -> Result<Option<String>, String> {
    if let Some(e) = entry(credential_ref) {
        match e.get_password() {
            Ok(s) => return Ok(Some(s)),
            Err(keyring::Error::NoEntry) => {}
            Err(_) => {}
        }
    }

    let store = load_file_store();
    Ok(store.get(credential_ref).cloned())
}

pub fn exists(credential_ref: &str) -> Result<bool, String> {
    if let Some(e) = entry(credential_ref) {
        if e.get_password().is_ok() {
            return Ok(true);
        }
    }

    let store = load_file_store();
    Ok(store.contains_key(credential_ref))
}

pub fn delete_secret(credential_ref: &str) -> Result<(), String> {
    if let Some(e) = entry(credential_ref) {
        let _ = e.delete_credential();
    }

    let mut store = load_file_store();
    store.remove(credential_ref);
    save_file_store(&store)?;

    Ok(())
}
