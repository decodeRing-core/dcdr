use std::fmt;

use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder, ResponseError};
use serde::Serialize;

use crate::error::ErrorReason;

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
    SystemLocked,
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
            Self::SystemUnlocked => "System unlocked",
            Self::SystemLocked => "System locked",
            Self::RaftInitialized => "Raft initialized",
            Self::RaftMetrics => "Raft node metrics",
            Self::RaftAddLearner => "Raft learner added",
            Self::RaftMembership => "Raft membership changes",
            Self::OperationCompleted => "Operation completed",
        }
    }

    fn http_status(&self) -> StatusCode {
        match self {
            Self::SystemInitialized
            | Self::SystemUnlocked
            | Self::SystemLocked
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
    OperationFailed(ErrorReason),
}

impl ErrorStatus {
    fn code(&self) -> &'static str {
        match self {
            Self::OperationFailed(_) => "operation-failed",
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::OperationFailed(_) => "Operation failed.",
        }
    }

    fn detail(&self) -> String {
        match self {
            Self::OperationFailed(detail) => detail.to_string(),
        }
    }

    fn http_status(&self) -> StatusCode {
        match self {
            Self::OperationFailed(reason) => reason.http_status(),
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
    detail: String,
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
                    detail: e.detail(),
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
