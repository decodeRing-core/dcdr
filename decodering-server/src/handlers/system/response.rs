use serde::Serialize;

use crate::handlers::response::{ApiResponse, SuccessStatus};

#[derive(Serialize)]
pub(crate) struct ApiInitSystemResponse {
    pub(crate) shards: Vec<String>,
    pub(crate) root_token: Option<String>,
}

impl ApiInitSystemResponse {
    pub(crate) fn initialized(
        shards: Vec<String>,
        root_token: Option<String>,
    ) -> ApiResponse<ApiInitSystemResponse> {
        ApiResponse::new(
            SuccessStatus::SystemInitialized.into(),
            Some(ApiInitSystemResponse { shards, root_token }),
        )
    }
}
