use serde::Serialize;

use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct ApiCreateAppUserResponse {
    pub(crate) token: String,
    pub(crate) principal_id: String,
}

impl ApiCreateAppUserResponse {
    pub(crate) fn new(token: String, principal_id: String) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self {
                token,
                principal_id,
            }),
        )
    }
}

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct ApiTpmChallengeResponse {
    pub(crate) challenge_id: String,
    pub(crate) nonce: String,
    pub(crate) expires_at: i64,
}

impl ApiTpmChallengeResponse {
    pub(crate) fn new(challenge_id: String, nonce: String, expires_at: i64) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self {
                challenge_id,
                nonce,
                expires_at,
            }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiCreateAppGrantResponse {}

impl ApiCreateAppGrantResponse {
    pub(crate) fn new() -> ApiResponse<Self> {
        ApiResponse::new(ApiStatus::Success(SuccessStatus::OperationCompleted), None)
    }
}
