use serde::Serialize;
use serde_json::Value;

use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};

#[derive(Serialize)]
pub(crate) struct ApiPutSecretResponse(String);

impl ApiPutSecretResponse {
    pub(crate) fn new(data: String) -> ApiResponse<ApiPutSecretResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiPutSecretResponse(data)),
        )
    }
}

#[derive(Serialize)]
pub(crate) struct ApiGetSecretResponse(Value);

impl ApiGetSecretResponse {
    pub(crate) fn new(data: Value) -> ApiResponse<ApiGetSecretResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiGetSecretResponse(data)),
        )
    }
}
