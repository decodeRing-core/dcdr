use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum DbError {
    NotFound,
    UniqueViolation { constraint: Option<String> },
    ForeignKeyViolation { constraint: Option<String> },
    CheckViolation { constraint: Option<String> },
    SerializationFailure,
    Connection(String),
    Schema(String),
    Serde(String),
    Other(String),
}

impl DbError {
    // True for errors that are deterministic outcomes of the command's
    // content (constraint violations, missing rows). These must NOT halt
    // the Raft apply loop — they're business failures, not storage failures.
    pub fn is_business(&self) -> bool {
        matches!(
            self,
            Self::NotFound
                | Self::UniqueViolation { .. }
                | Self::ForeignKeyViolation { .. }
                | Self::CheckViolation { .. }
        )
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "not found"),
            Self::UniqueViolation {
                constraint: Some(c),
            } => {
                write!(f, "unique constraint violated: {c}")
            }
            Self::UniqueViolation { constraint: None } => {
                write!(f, "unique constraint violated")
            }
            Self::ForeignKeyViolation {
                constraint: Some(c),
            } => {
                write!(f, "foreign key constraint violated: {c}")
            }
            Self::ForeignKeyViolation { constraint: None } => {
                write!(f, "foreign key constraint violated")
            }
            Self::CheckViolation {
                constraint: Some(c),
            } => {
                write!(f, "check constraint violated: {c}")
            }
            Self::CheckViolation { constraint: None } => {
                write!(f, "check constraint violated")
            }
            Self::SerializationFailure => write!(f, "serialization failure; retry"),
            Self::Connection(msg) => write!(f, "connection error: {msg}"),
            Self::Schema(msg) => write!(f, "schema error: {msg}"),
            Self::Serde(msg) => write!(f, "serialization error: {msg}"),
            Self::Other(msg) => write!(f, "database error: {msg}"),
        }
    }
}
impl Error for DbError {}

// Reason a policy check denied an action.
#[derive(Debug, Clone)]
pub struct DenyReason(pub String);

impl fmt::Display for DenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error from `Action::execute`. Wraps `DbError` plus action-specific failures.
#[derive(Debug)]
pub enum ExecutionError {
    Db(DbError),
    Other(String),
}

impl ExecutionError {
    pub fn is_business(&self) -> bool {
        match self {
            Self::Db(e) => e.is_business(),
            Self::Other(_) => false, // unknown — treat as fatal
        }
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => write!(f, "db error: {e}"),
            Self::Other(s) => write!(f, "execution error: {s}"),
        }
    }
}

impl Error for ExecutionError {}

impl From<DbError> for ExecutionError {
    fn from(e: DbError) -> Self {
        Self::Db(e)
    }
}

#[derive(Debug)]
pub enum ActionError {
    Db(DbError),
    Denied(DenyReason),
    Execution(ExecutionError),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => write!(f, "database error: {e}"),
            Self::Denied(reason) => write!(f, "action denied: {reason}"),
            Self::Execution(e) => write!(f, "execution error: {e}"),
        }
    }
}
