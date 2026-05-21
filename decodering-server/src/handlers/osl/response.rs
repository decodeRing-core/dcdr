use decodering_core::{
    plugin::{
        orchestrator::BackendCapabilities,
        osl_contract::{Capability, DescribeOutput},
    },
    repository::SecretMapping,
};
use serde::Serialize;
use serde_json::Value;

use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};

#[derive(Serialize)]
pub struct ApiPutSecretResponse {
    pub secret_name: String,
    pub provider_version_id: String,
}

impl ApiPutSecretResponse {
    pub(crate) fn new(secret_name: String, provider_version_id: String) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self {
                secret_name,
                provider_version_id,
            }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiGetSecretResponse {
    #[serde(flatten)]
    data: Value,
    metadata: ApiGetSecretMetadataResponse,
}

#[derive(Serialize)]
struct ApiGetSecretMetadataResponse {
    resolved_backend_ref: String,
    provider_version_id: String,
}

impl ApiGetSecretResponse {
    pub(crate) fn new(
        data: Value,
        resolved_backend_ref: String,
        provider_version_id: String,
    ) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self {
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
pub struct ApiDestroySecretResponse {
    pub(crate) destroyed: bool,
}

impl ApiDestroySecretResponse {
    pub(crate) fn new(destroyed: bool) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { destroyed }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiDeleteSecretResponse {
    pub(crate) soft_deleted: bool,
}

impl ApiDeleteSecretResponse {
    pub(crate) fn new(soft_deleted: bool) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { soft_deleted }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiListSecretResponse(Vec<ListSecretResponse>);

#[derive(Serialize)]
struct ListSecretResponse {
    secret_name: String,
    backend: String,
    mount_path: String,
    tainted: bool,
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
    pub(crate) fn new(secrets: Vec<SecretMapping>) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self(secrets.into_iter().map(Into::into).collect())),
        )
    }
}

#[derive(Serialize)]
pub struct ApiTaintSecretResponse {
    pub(crate) tainted: bool,
}

impl ApiTaintSecretResponse {
    pub(crate) fn new(tainted: bool) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { tainted }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiIsTaintedSecretResponse {
    pub(crate) is_tainted: bool,
}

impl ApiIsTaintedSecretResponse {
    pub(crate) fn new(is_tainted: bool) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { is_tainted }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiRestoreSecretResponse {
    pub(crate) restored: bool,
}

impl ApiRestoreSecretResponse {
    pub(crate) fn new(restored: bool) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { restored }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiCapabilitiesResponse {
    pub(crate) server_capabilities: Vec<Capability>,
    pub(crate) backends: Vec<BackendCapabilities>,
}

impl ApiCapabilitiesResponse {
    pub(crate) fn new(
        server_capabilities: Vec<Capability>,
        backends: Vec<BackendCapabilities>,
    ) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self {
                server_capabilities,
                backends,
            }),
        )
    }
}

#[derive(Serialize)]
pub struct ApiDescribeSecretResponse {
    #[serde(flatten)]
    pub(crate) output: DescribeOutput,
}

impl ApiDescribeSecretResponse {
    pub(crate) fn new(output: DescribeOutput) -> ApiResponse<Self> {
        ApiResponse::new(
            ApiStatus::Success(SuccessStatus::OperationCompleted),
            Some(Self { output }),
        )
    }
}
