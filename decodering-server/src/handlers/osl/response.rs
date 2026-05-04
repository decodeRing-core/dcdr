use serde::Serialize;
use serde_json::Value;

use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};

#[derive(Serialize)]
pub(crate) struct ApiPutSecretResponse {
    pub secret_name: String,
    pub provider_version_id: String,
}

impl ApiPutSecretResponse {
    pub(crate) fn new(
        secret_name: String,
        provider_version_id: String,
    ) -> ApiResponse<ApiPutSecretResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiPutSecretResponse {
                secret_name,
                provider_version_id,
            }),
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
