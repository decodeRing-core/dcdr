use std::error::Error;
use std::fmt;

use decodering_db::DbError;

/// Reason a policy check denied an action.
#[derive(Debug, Clone)]
pub struct DenyReason(pub String);

impl fmt::Display for DenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error from `Action::execute`. Wraps DbError plus action-specific failures.
#[derive(Debug)]
pub enum ExecutionError {
    Db(DbError),
    Other(String),
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Db(e) => write!(f, "db error: {}", e),
            Self::Other(s) => write!(f, "execution error: {}", s),
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
