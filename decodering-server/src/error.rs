use decodering_core::error::ActionError;
use decodering_raft::raft_types::{ClientWriteError, RaftError};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Action(ActionError),
    Raft(RaftError<ClientWriteError>),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Action(e) => write!(f, "action error: {e}"),
            Self::Raft(e) => write!(f, "raft error: {e}"),
        }
    }
}

impl Error for AppError {}

impl From<ActionError> for AppError {
    fn from(e: ActionError) -> Self {
        Self::Action(e)
    }
}
