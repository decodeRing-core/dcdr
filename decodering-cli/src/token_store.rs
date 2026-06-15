use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use keyring_core::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "decodering-cli";
const ROOT: &str = "root-token";
const SESSION: &str = "session-token";
const LEEWAY_SECS: i64 = 5;

pub enum StoredIn {
    Keyring,
    File(PathBuf),
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub expires_at: i64,
}

pub fn store(secret: &str) -> Result<StoredIn, Box<dyn Error>> {
    store_value(ROOT, secret)
}

pub fn load() -> Result<Option<String>, Box<dyn Error>> {
    if let Ok(token) = std::env::var("DECODERING_TOKEN") {
        return Ok(Some(token));
    }
    load_value(ROOT)
}

pub fn _clear() -> Result<(), Box<dyn Error>> {
    clear_value(ROOT)
}

pub fn save_session(session: &Session) -> Result<(), Box<dyn Error>> {
    store_value(SESSION, &serde_json::to_string(session)?)?;
    Ok(())
}

pub fn load_session() -> Option<Session> {
    let raw = load_value(SESSION).ok().flatten()?;
    let session: Session = serde_json::from_str(&raw).ok()?;
    if session.expires_at <= now() + LEEWAY_SECS {
        let _ = clear_value(SESSION);
        return None;
    }
    Some(session)
}

pub fn _clear_session() -> Result<(), Box<dyn Error>> {
    clear_value(SESSION)
}

/// Token to use for OSL endpoints: valid session, else root, else error.
pub fn resolve_token() -> Result<String, Box<dyn Error>> {
    if let Some(session) = load_session() {
        return Ok(session.token);
    }
    if let Some(root) = load()? {
        return Ok(root);
    }
    Err("no valid session or root token; run `app user auth` or `system init`".into())
}

pub fn release() {
    keyring_core::unset_default_store();
}

fn store_value(name: &str, value: &str) -> Result<StoredIn, Box<dyn Error>> {
    if keyring_available()
        && Entry::new(SERVICE, name)
            .and_then(|e| e.set_password(value))
            .is_ok()
    {
        return Ok(StoredIn::Keyring);
    }
    Ok(StoredIn::File(store_in_file(name, value)?))
}

fn load_value(name: &str) -> Result<Option<String>, Box<dyn Error>> {
    if keyring_available()
        && let Ok(entry) = Entry::new(SERVICE, name)
        && let Ok(v) = entry.get_password()
    {
        return Ok(Some(v));
    }
    let path = file_path(name)?;
    if path.exists() {
        return Ok(Some(fs::read_to_string(&path)?.trim().to_owned()));
    }
    Ok(None)
}

fn clear_value(name: &str) -> Result<(), Box<dyn Error>> {
    if keyring_available()
        && let Ok(entry) = Entry::new(SERVICE, name)
    {
        let _ = entry.delete_credential();
    }
    let path = file_path(name)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(0)
}

fn keyring_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| set_native_store().is_ok())
}

#[cfg(target_os = "macos")]
fn set_native_store() -> keyring_core::Result<()> {
    use apple_native_keyring_store::keychain::Store;
    keyring_core::set_default_store(Store::new_with_configuration(&HashMap::new())?);
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_native_store() -> keyring_core::Result<()> {
    use windows_native_keyring_store::Store;
    keyring_core::set_default_store(Store::new_with_configuration(&HashMap::new())?);
    Ok(())
}

#[cfg(target_os = "linux")]
fn set_native_store() -> keyring_core::Result<()> {
    use zbus_secret_service_keyring_store::Store;
    keyring_core::set_default_store(Store::new_with_configuration(&HashMap::new())?);
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn set_native_store() -> keyring_core::Result<()> {
    Err(KeyringError::NotSupportedByStore(
        "no native credential store on this platform".into(),
    ))
}

fn store_in_file(name: &str, value: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = file_path(name)?;
    let dir = path.parent().ok_or("invalid path")?;
    fs::create_dir_all(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        fs::write(&path, value)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    fs::write(&path, value)?;

    Ok(path)
}

fn file_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    Ok(dirs::home_dir()
        .ok_or("cannot determine home directory")?
        .join(".decodering-cli")
        .join(name))
}
