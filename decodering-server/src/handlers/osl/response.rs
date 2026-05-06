use decodering_core::repository::SecretMapping;
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
struct ApiGetSecretMetadataResponse {
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

#[derive(Serialize)]
pub(crate) struct ApiDestroySecretResponse {
    pub(crate) destroyed: bool,
}

impl ApiDestroySecretResponse {
    pub(crate) fn new(destroyed: bool) -> ApiResponse<ApiDestroySecretResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiDestroySecretResponse { destroyed }),
        )
    }
}

#[derive(Serialize)]
pub(crate) struct ApiListSecretResponse(Vec<ListSecretResponse>);

#[derive(Serialize)]
struct ListSecretResponse {
    pub(crate) secret_name: String,
    pub(crate) backend: String,
    pub(crate) mount_path: String,
    pub(crate) tainted: bool,
}

impl From<SecretMapping> for ListSecretResponse {
    fn from(value: SecretMapping) -> Self {
        Self {
            secret_name: value.secret_name,
            backend: value.backend,
            mount_path: value.mount_path,
            tainted: value.tainted == 1,
        }
    }
}

impl ApiListSecretResponse {
    pub(crate) fn new(secrets: Vec<SecretMapping>) -> ApiResponse<ApiListSecretResponse> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(ApiListSecretResponse(
                secrets.into_iter().map(|f| f.into()).collect(),
            )),
        )
    }
}
