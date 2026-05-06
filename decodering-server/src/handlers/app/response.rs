use serde::Serialize;

use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};

#[derive(Serialize)]
pub(crate) struct ApiCreateAppResponse {
    pub(crate) app_id: String,
    pub(crate) app_name: String,
}

impl ApiCreateAppResponse {
    pub(crate) fn new(app_id: String, app_name: String) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiCreateAppResponse { app_id, app_name }),
        )
    }
}

#[derive(Serialize)]
pub(crate) struct ApiCreateAppUserResponse {
    pub(crate) token: String,
}

impl ApiCreateAppUserResponse {
    pub(crate) fn new(token: String) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiCreateAppUserResponse { token }),
        )
    }
}
