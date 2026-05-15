use actix_web::http::StatusCode;
use decodering_core::error::ActionError;
use decodering_raft::raft_types::{ClientWriteError, RaftError};
use std::borrow::Cow;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorReason {
    Unexpected,
    GenericFail(Cow<'static, str>),
    Database,
    MissingData(&'static str),
    CertVerification,
    CertMissing(&'static str),
    InvalidPublicKey,
    Internal,
    TrustStore,
    ApplicationNotFound,
    PrincipalNotFound,
    SecretAlreadyExists,
    SystemNotInitialized,
    AlreadyInitialized,
    Plugin,
    Locked,
    InvalidShamirKeys,
    UnsupportedBackend,
    SecretNotFound,
    Unauthorized,
    Unimplemented,
    DuplicatedApp,
    ChallengeMismatch,
    RaftNotLeader,
    RaftNotInitialized,
    RaftNotAvailable,
    Raft,
}

impl std::fmt::Display for ErrorReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unexpected => f.write_str("Unexpected response."),
            Self::Internal => f.write_str("Internal error."),
            Self::GenericFail(name) => write!(f, "Failed to {name}."),
            Self::Database => f.write_str("Database error."),
            Self::CertVerification => f.write_str("Certification verification failed."),
            Self::TrustStore => f.write_str("Failed to load trust store."),
            Self::CertMissing(name) => write!(f, "Missing cert {name}."),
            Self::MissingData(name) => write!(f, "Missing {name} data."),
            Self::InvalidPublicKey => f.write_str("Invalid public key."),
            Self::PrincipalNotFound => f.write_str("Principal not found."),
            Self::ApplicationNotFound => f.write_str("Application not found."),
            Self::SecretAlreadyExists => f.write_str("Secret already exists."),
            Self::RaftNotLeader => f.write_str("Node is not the leader."),
            Self::RaftNotInitialized => f.write_str("Node is not initialized."),
            Self::AlreadyInitialized => f.write_str("System already initialized."),
            Self::Plugin => f.write_str("Plugin error."),
            Self::InvalidShamirKeys => f.write_str("Invalid shamir keys."),
            Self::UnsupportedBackend => f.write_str("Unsupported backend."),
            Self::SecretNotFound => f.write_str("Secret not found."),
            Self::Unauthorized => f.write_str("Unauthorized access."),
            Self::Unimplemented => f.write_str("Not implemented."),
            Self::DuplicatedApp => f.write_str("Application with the same name already exists."),
            Self::ChallengeMismatch => f.write_str("Challenge mismatch."),
            Self::RaftNotAvailable => f.write_str("Raft not configured."),
            Self::Raft => f.write_str("Raft error."),
            Self::Locked => f.write_str("System locked."),
            Self::SystemNotInitialized => f.write_str("System not initialized."),
        }
    }
}

impl ErrorReason {
    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::Unexpected
            | Self::GenericFail(_)
            | Self::Database
            | Self::Internal
            | Self::TrustStore
            | Self::Raft => StatusCode::INTERNAL_SERVER_ERROR,
            Self::MissingData(_)
            | Self::CertVerification
            | Self::CertMissing(_)
            | Self::InvalidPublicKey
            | Self::ApplicationNotFound
            | Self::PrincipalNotFound
            | Self::SecretAlreadyExists
            | Self::AlreadyInitialized
            | Self::Plugin
            | Self::ChallengeMismatch
            | Self::RaftNotAvailable => StatusCode::BAD_REQUEST,
            Self::RaftNotLeader => StatusCode::MISDIRECTED_REQUEST,
            Self::RaftNotInitialized | Self::SystemNotInitialized => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            Self::InvalidShamirKeys | Self::Unauthorized | Self::Locked => StatusCode::FORBIDDEN,
            Self::UnsupportedBackend | Self::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            Self::SecretNotFound => StatusCode::NOT_FOUND,
            Self::DuplicatedApp => StatusCode::CONFLICT,
        }
    }
}
