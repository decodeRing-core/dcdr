use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use keyring_core::Entry;

const SERVICE: &str = "yourcli";
const ACCOUNT: &str = "root-key";

pub enum StoredIn {
    Keyring,
    File(PathBuf),
}

pub fn store(secret: &str) -> Result<StoredIn, Box<dyn Error>> {
    if keyring_available()
        && matches!(
            Entry::new(SERVICE, ACCOUNT).and_then(|e| e.set_password(secret)),
            Ok(())
        )
    {
        return Ok(StoredIn::Keyring);
    }
    Ok(StoredIn::File(store_in_file(secret)?))
}

pub fn load() -> Result<Option<String>, Box<dyn Error>> {
    if let Ok(token) = std::env::var("YOURCLI_TOKEN") {
        return Ok(Some(token));
    }
    if keyring_available()
        && let Ok(entry) = Entry::new(SERVICE, ACCOUNT)
        && let Ok(token) = entry.get_password()
    {
        return Ok(Some(token));
    }
    let path = token_file_path()?;
    if path.exists() {
        return Ok(Some(fs::read_to_string(&path)?.trim().to_owned()));
    }
    Ok(None)
}

pub fn clear() -> Result<(), Box<dyn Error>> {
    if keyring_available()
        && let Ok(entry) = Entry::new(SERVICE, ACCOUNT)
    {
        let _ = entry.delete_credential();
    }
    let path = token_file_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn release() {
    keyring_core::unset_default_store();
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

fn store_in_file(secret: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path = token_file_path()?;
    let dir = path.parent().ok_or("invalid token path")?;
    fs::create_dir_all(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        fs::write(&path, secret)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    fs::write(&path, secret)?;

    Ok(path)
}

fn token_file_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(dirs::home_dir()
        .ok_or("cannot determine home directory")?
        .join(".yourcli")
        .join("token"))
}
