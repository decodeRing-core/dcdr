use serde::Serialize;

use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};

#[derive(Serialize)]
pub(crate) struct ApiCreateAppResponse {
    pub(crate) data: String,
}

impl ApiCreateAppResponse {
    pub(crate) fn new(data: String) -> ApiResponse<ApiCreateAppResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiCreateAppResponse { data }),
        )
    }
}
