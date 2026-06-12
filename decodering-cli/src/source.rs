use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone)]
pub enum ValueSource {
    Inline(String),
    File(PathBuf),
}

impl FromStr for ValueSource {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.strip_prefix('@').map_or_else(
            || Self::Inline(s.to_owned()),
            |path| Self::File(PathBuf::from(path)),
        ))
    }
}

impl ValueSource {
    pub fn read(&self) -> Result<String, Box<dyn Error>> {
        Ok(match self {
            Self::Inline(s) => s.clone(),
            Self::File(path) => fs::read_to_string(path)?,
        })
    }
}

#[derive(Clone)]
pub enum SecretSource {
    File(PathBuf),
    Stdin,
}

impl FromStr for SecretSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            Ok(Self::Stdin)
        } else if let Some(path) = s.strip_prefix('@') {
            Ok(Self::File(PathBuf::from(path)))
        } else {
            Err("credentials must come from a file (`@path`) or stdin (`-`)".into())
        }
    }
}

impl SecretSource {
    pub fn read(&self) -> Result<String, Box<dyn Error>> {
        Ok(match self {
            Self::File(path) => fs::read_to_string(path)?,
            Self::Stdin => {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            }
        })
    }
}
