use std::fmt;

use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder, ResponseError};
use serde::Serialize;

const OSL_VERSION: &str = "1.0.0";

#[derive(Debug, Clone)]
pub enum ApiStatus {
    Success(SuccessStatus),
    Error(ErrorStatus),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SuccessStatus {
    SystemInitialized,
    SystemUnlocked,
    RaftInitialized,
    RaftMetrics,
    RaftAddLearner,
    RaftMembership,
    OperationCompleted,
}

impl SuccessStatus {
    fn message(&self) -> &'static str {
        match self {
            Self::SystemInitialized => "System initialized",
            Self::SystemUnlocked => "System is unlocked",
            Self::RaftInitialized => "Raft initialized",
            Self::RaftMetrics => "Raft node metrics",
            Self::RaftAddLearner => "Raft learner added",
            Self::RaftMembership => "Raft membership changes",
            Self::OperationCompleted => "Operating completed",
        }
    }

    fn http_status(&self) -> StatusCode {
        match self {
            Self::SystemInitialized
            | Self::SystemUnlocked
            | Self::RaftInitialized
            | Self::RaftMetrics
            | Self::RaftAddLearner
            | Self::RaftMembership
            | Self::OperationCompleted => StatusCode::OK,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ErrorStatus {
    NotLeader,
    NotInitialized,
    AlreadyInitialized,
    Plugin,
    Internal,
    InvalidKeys,
    Locked,
    UnsupportedBackend,
    SecretNotFound,
    Unauthorized,
    Unimplemented,
    DuplicatedApp,
    ChallengeMismatch,
}

impl ErrorStatus {
    fn code(&self) -> &'static str {
        match self {
            Self::NotLeader => "not-leader",
            Self::NotInitialized => "node-not-initialized",
            Self::AlreadyInitialized => "system-already-initialized",
            Self::Plugin => "plugin-error",
            Self::Internal => "internal-error",
            Self::InvalidKeys => "invalid-keys",
            Self::Locked => "locked",
            Self::UnsupportedBackend => "unsupported-backend",
            Self::SecretNotFound => "secret-not-found",
            Self::Unauthorized => "unauthorized",
            Self::Unimplemented => "not-implemented",
            Self::DuplicatedApp => "duplicated-app",
            Self::ChallengeMismatch => "challenge-mismatch",
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::NotLeader => "Node is not the leader.",
            Self::NotInitialized => "Node is not initialized.",
            Self::AlreadyInitialized => "System already initialized.",
            Self::Plugin => "Plugin error.",
            Self::Internal => "Internal error.",
            Self::InvalidKeys => "Invalid keys.",
            Self::Locked => "Node locked.",
            Self::UnsupportedBackend => "Unsupported backend.",
            Self::SecretNotFound => "Secret not found.",
            Self::Unauthorized => "Unauthorized access.",
            Self::Unimplemented => "Not implemented.",
            Self::DuplicatedApp => "App with the same name already exists.",
            Self::ChallengeMismatch => "Challenge mismatch.",
        }
    }

    fn http_status(&self) -> StatusCode {
        match self {
            Self::NotLeader => StatusCode::MISDIRECTED_REQUEST,
            Self::NotInitialized => StatusCode::SERVICE_UNAVAILABLE,
            Self::AlreadyInitialized | Self::Plugin => StatusCode::BAD_REQUEST,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidKeys | Self::Locked | Self::Unauthorized => StatusCode::FORBIDDEN,
            Self::UnsupportedBackend | Self::Unimplemented => StatusCode::NOT_IMPLEMENTED,
            Self::SecretNotFound => StatusCode::NOT_FOUND,
            Self::DuplicatedApp => StatusCode::CONFLICT,
            Self::ChallengeMismatch => StatusCode::BAD_REQUEST,
        }
    }
}

impl From<SuccessStatus> for ApiStatus {
    fn from(s: SuccessStatus) -> Self {
        Self::Success(s)
    }
}
impl From<ErrorStatus> for ApiStatus {
    fn from(e: ErrorStatus) -> Self {
        Self::Error(e)
    }
}

impl fmt::Display for ErrorStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for ErrorStatus {}

impl ResponseError for ErrorStatus {
    fn status_code(&self) -> StatusCode {
        self.http_status()
    }

    fn error_response(&self) -> HttpResponse {
        let body = ApiResponse::<()>::error(self.clone());
        let http_status = body.http_status;
        match serde_json::to_string(&body) {
            Ok(json) => HttpResponse::build(http_status)
                .content_type("application/json")
                .body(json),
            Err(e) => HttpResponse::InternalServerError().body(format!("serialization error: {e}")),
        }
    }
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    osl_version: &'static str,

    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<SuccessStatus>,

    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'static str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,

    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ApiErrorBody>,

    #[serde(skip)]
    http_status: StatusCode,
}

#[derive(Serialize)]
pub struct ApiErrorBody {
    code: &'static str,
    message: &'static str,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn new(status: ApiStatus, data: Option<T>) -> Self {
        match status {
            ApiStatus::Success(s) => Self {
                osl_version: OSL_VERSION,
                message: Some(s.message()),
                http_status: s.http_status(),
                status: Some(s),
                data,
                error: None,
            },
            ApiStatus::Error(e) => Self {
                osl_version: OSL_VERSION,
                status: None,
                message: None,
                data: None,
                http_status: e.http_status(),
                error: Some(ApiErrorBody {
                    code: e.code(),
                    message: e.message(),
                }),
            },
        }
    }

    pub fn error(status: ErrorStatus) -> Self {
        Self::new(ApiStatus::Error(status), None)
    }
    pub fn empty(status: ApiStatus) -> Self {
        Self::new(status, None)
    }
}

impl<T: Serialize> ApiResponse<T> {}

impl<T: Serialize> Responder for ApiResponse<T> {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let http_status = self.http_status;
        match serde_json::to_string(&self) {
            Ok(body) => HttpResponse::build(http_status)
                .content_type("application/json")
                .body(body),
            Err(e) => HttpResponse::InternalServerError().body(format!("serialization error: {e}")),
        }
    }
}
