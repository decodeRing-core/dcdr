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
pub(crate) struct ApiGetSecretResponse {
    #[serde(flatten)]
    pub(crate) data: Value,
    pub(crate) metadata: ApiGetSecretMetadataResponse,
}

#[derive(Serialize)]
pub(crate) struct ApiGetSecretMetadataResponse {
    pub(crate) resolved_backend_ref: String,
    pub(crate) provider_version_id: String,
}

impl ApiGetSecretResponse {
    pub(crate) fn new(
        data: Value,
        resolved_backend_ref: String,
        provider_version_id: String,
    ) -> ApiResponse<ApiGetSecretResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiGetSecretResponse {
                data,
                metadata: ApiGetSecretMetadataResponse {
                    resolved_backend_ref,
                    provider_version_id,
                },
            }),
        )
    }
}
