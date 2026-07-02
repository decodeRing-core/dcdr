use serde::Serialize;
use utoipa::ToSchema;

use crate::handlers::response::{ApiResponse, SuccessStatus};

#[derive(Serialize, ToSchema)]
pub struct ApiInitSystemResponse {
    pub(crate) shards: Vec<String>,
    pub(crate) root_token: Option<String>,
}

impl ApiInitSystemResponse {
    pub(crate) fn initialized(
        shards: Vec<String>,
        root_token: Option<String>,
    ) -> ApiResponse<Self> {
        ApiResponse::new(
            SuccessStatus::SystemInitialized.into(),
            Some(Self { shards, root_token }),
        )
    }
}

#[derive(Serialize, ToSchema)]
pub struct ApiSystemStatusResponse {
    pub(crate) initialized: bool,
    pub(crate) unlocked: bool,
}

impl ApiSystemStatusResponse {
    pub(crate) fn new(initialized: bool, unlocked: bool) -> ApiResponse<Self> {
        ApiResponse::new(
            SuccessStatus::SystemStatus.into(),
            Some(Self {
                initialized,
                unlocked,
            }),
        )
    }
}
