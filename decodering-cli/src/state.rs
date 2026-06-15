use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn state_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    Ok(dirs::home_dir()
        .ok_or("cannot determine home directory")?
        .join(".decodering-cli")
        .join(name))
}

fn read_value(name: &str) -> Option<String> {
    let value = fs::read_to_string(state_path(name).ok()?).ok()?;
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn write_value(name: &str, value: &str) -> Result<(), Box<dyn Error>> {
    let path = state_path(name)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, value)?;
    Ok(())
}

pub fn last_principal() -> Option<String> {
    read_value("last-principal")
}

pub fn set_last_principal(id: &str) -> Result<(), Box<dyn Error>> {
    write_value("last-principal", id)
}

pub fn last_app() -> Option<String> {
    read_value("last-app")
}

pub fn set_last_app(id: &str) -> Result<(), Box<dyn Error>> {
    write_value("last-app", id)
}
