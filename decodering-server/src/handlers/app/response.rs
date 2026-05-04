use serde::Serialize;

use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};

#[derive(Serialize)]
pub(crate) struct ApiCreateAppResponse {
    pub(crate) app_id: String,
    pub(crate) app_name: String,
}

impl ApiCreateAppResponse {
    pub(crate) fn new(app_id: String, app_name: String) -> ApiResponse<ApiCreateAppResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiCreateAppResponse { app_id, app_name }),
        )
    }
}
