use keyring::Entry;

const SERVICE_NAME: &str = "cutline";

fn entry(credential_ref: &str) -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, credential_ref).map_err(|e| format!("keychain_unavailable: {}", e))
}

pub fn set_secret(credential_ref: &str, secret: &str) -> Result<(), String> {
    entry(credential_ref)?
        .set_password(secret)
        .map_err(|e| format!("keychain_write_error: {}", e))
}

pub fn get_secret(credential_ref: &str) -> Result<Option<String>, String> {
    match entry(credential_ref)?.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keychain_read_error: {}", e)),
    }
}

pub fn exists(credential_ref: &str) -> Result<bool, String> {
    match entry(credential_ref)?.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(format!("keychain_read_error: {}", e)),
    }
}

pub fn delete_secret(credential_ref: &str) -> Result<(), String> {
    match entry(credential_ref)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keychain_delete_error: {}", e)),
    }
}
