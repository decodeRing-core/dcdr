use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum PluginError {
    BackendNotFound(String),
    Instantiation(String),
    Call { function: String, message: String },
    Serde(String),
    Io(String),
    Unimplemented(String),
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendNotFound(b) => write!(f, "backend not found: {b}"),
            Self::Instantiation(m) => write!(f, "plugin instantiation failed: {m}"),
            Self::Call { function, message } => {
                write!(f, "plugin call '{function}' failed: {message}")
            }
            Self::Serde(m) => write!(f, "plugin serde error: {m}"),
            Self::Io(m) => write!(f, "plugin io error: {m}"),
            Self::Unimplemented(m) => write!(f, "Not implemented: {m}"),
        }
    }
}

impl Error for PluginError {}
