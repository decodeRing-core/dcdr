use crate::handlers::response::{ApiResponse, ApiStatus, SuccessStatus};
use serde::Serialize;
use serde_json::Value;
use serde_with::base64::Base64;
use serde_with::serde_as;

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
    #[serde(flatten)]
    pub(crate) payload: Option<Value>,
    pub(crate) principal_id: String,
    pub(crate) credential_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tpm: Option<TpmChallengeData>,
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
                tpm: None,
            }),
        )
    }

    // pub(crate) fn tpm(
    //     token: String,
    //     principal_id: String,
    //     credential_id: String,
    //     tpm: Option<TpmChallengeData>,
    // ) -> ApiResponse<Self> {
    //     ApiResponse::new(
    //         ApiStatus::Success(SuccessStatus::OperationCompleted),
    //         Some(Self {
    //             token,
    //             principal_id,
    //             credential_id,
    //             tpm,
    //         }),
    //     )
    // }
}

#[serde_as]
#[derive(Serialize, Default)]
pub struct TpmChallengeData {
    #[serde_as(as = "Base64")]
    pub credential_blob: Vec<u8>,
    #[serde_as(as = "Base64")]
    pub secret: Vec<u8>,
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

#[derive(Serialize)]
pub struct ApiDeleteAppGrantResponse {}

impl ApiDeleteAppGrantResponse {
    pub(crate) fn new() -> ApiResponse<Self> {
        ApiResponse::new(ApiStatus::Success(SuccessStatus::OperationCompleted), None)
    }
}
