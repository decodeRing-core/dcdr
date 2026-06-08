use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ApiCreateAppResponse {
    pub(crate) app_id: String,
    pub(crate) app_name: String,
}

impl ApiCreateAppResponse {
    pub(crate) fn new(app_id: String, app_name: String) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { app_id, app_name }),
        )
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiCreateAppUserResponse {
    #[serde(flatten)]
    pub(crate) payload: Option<Value>,
    pub(crate) principal_id: String,
    pub(crate) credential_id: String,
}

impl ApiCreateAppUserResponse {
    pub(crate) fn new(
        payload: Option<Value>,
        principal_id: String,
        credential_id: String,
    ) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self {
                payload,
                principal_id,
                credential_id,
            }),
        )
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiAuthAppUserResponse {
    pub(crate) token: String,
    pub(crate) expires_at: i64,
}

impl ApiAuthAppUserResponse {
    pub(crate) fn new(token: String, expires_at: i64) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { token, expires_at }),
        )
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiAuthChallengeResponse {
    pub(crate) challenge_id: String,
    #[serde(flatten)]
    pub(crate) payload: Value,
    pub(crate) expires_at: i64,
}

impl ApiAuthChallengeResponse {
    pub(crate) fn new(challenge_id: String, payload: Value, expires_at: i64) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self {
                challenge_id,
                payload,
                expires_at,
            }),
        )
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiCreateAppGrantResponse {}

impl ApiCreateAppGrantResponse {
    pub(crate) fn new() -> ApiResponse<Self> {
        ApiResponse::new(ApiStatus::Success(SuccessStatus::OperationCompleted), None)
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiDeleteAppGrantResponse {}

impl ApiDeleteAppGrantResponse {
    pub(crate) fn new() -> ApiResponse<Self> {
        ApiResponse::new(ApiStatus::Success(SuccessStatus::OperationCompleted), None)
    }
}
