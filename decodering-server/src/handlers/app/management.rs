use actix_web::Responder;
use actix_web::web::{Data, Json};
use decodering_core::actions::create_app::CreateApp;
use decodering_core::response::AppResponse;
use decodering_core::tx::Database;
use uuid::Uuid;

use crate::app_data::AppData;
use crate::extractor::AuthMiddleware;
use crate::handlers::app::payload::{CreateAppData, CreateAppUserData};
use crate::handlers::app::response::ApiCreateAppResponse;
use crate::handlers::response::{ApiResponse, ErrorStatus};

pub(crate) async fn create_app_user<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<CreateAppUserData>,
    auth: AuthMiddleware<D>,
) -> impl Responder {
    if !auth.user.is_admin {
        return ApiResponse::error(ErrorStatus::Unauthorized.into());
    }
    match &app.raft {
        Some(raft_bits) => {
            let is_initialized = raft_bits.raft.is_initialized().await;
            if !matches!(is_initialized, Ok(true)) {
                return ApiResponse::error(ErrorStatus::NotInitialized.into());
            }
            if !raft_bits.raft.is_leader() {
                return ApiResponse::error(ErrorStatus::NotLeader.into());
            }
        }
        _ => {}
    }

    ApiResponse::<()>::error(ErrorStatus::Internal.into())
}

pub(crate) async fn create_app<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<CreateAppData>,
    auth: AuthMiddleware<D>,
) -> impl Responder {
    if !auth.user.is_admin {
        return ApiResponse::error(ErrorStatus::Unauthorized.into());
    }
    let request = CreateApp::request(Uuid::now_v7().to_string(), req.0.app_name);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreateApp(a) => ApiCreateAppResponse::new(a.app_id, a.app_name),
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to create app");
                return ApiResponse::error(ErrorStatus::Internal.into());
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                return ApiResponse::error(ErrorStatus::Internal.into());
            }
        },
        Err(e) => {
            tracing::error!(?e);
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    }
}
