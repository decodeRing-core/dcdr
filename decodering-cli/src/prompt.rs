use std::error::Error;
use std::str::FromStr;

/// Optional single-line input; empty string is allowed (used as a loop sentinel).
pub fn line(label: &str) -> Result<String, Box<dyn Error>> {
    let value: String = cliclack::input(label).required(false).interact()?;
    Ok(value.trim().to_owned())
}

/// Required single-line input.
pub fn required(label: &str) -> Result<String, Box<dyn Error>> {
    let value: String = cliclack::input(label).interact()?;
    Ok(value.trim().to_owned())
}

/// Input with a default applied when the user submits nothing.
pub fn with_default(label: &str, default: &str) -> Result<String, Box<dyn Error>> {
    let value: String = cliclack::input(label).default_input(default).interact()?;
    Ok(value.trim().to_owned())
}

/// Typed input; cliclack re-prompts until it parses.
pub fn parse<T: FromStr>(label: &str) -> Result<T, Box<dyn Error>> {
    Ok(cliclack::input(label).interact::<T>()?)
}

/// Hidden input for secrets.
pub fn password(label: &str) -> Result<String, Box<dyn Error>> {
    let value = cliclack::password(label).interact()?;
    Ok(value.trim().to_owned())
}

pub fn confirm(label: &str) -> Result<bool, Box<dyn Error>> {
    Ok(cliclack::confirm(label).interact()?)
}
