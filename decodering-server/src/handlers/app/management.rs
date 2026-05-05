use actix_web::Responder;
use actix_web::web::{Data, Json};
use decodering_core::actions::create_app::CreateApp;
use decodering_core::actions::create_principal::CreatePrincipal;
use decodering_core::actions::create_principal_credential::CreatePrincipalCredential;
use decodering_core::domain::{PrincipalCredentialKind, PrincipalStatus};
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::tx::Database;
use decodering_core::{now_ts, sha256_hex};
use rand::distr::{Alphanumeric, SampleString};
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

    let timestamp = now_ts();
    let principal_id = Uuid::now_v7().to_string();
    let principal = CreatePrincipal {
        principal_id: principal_id.clone(),
        name: req.0.name,
        app_id: req.0.app_id,
        kind: req.0.kind,
        status: PrincipalStatus::Active,
        created_at: timestamp,
        updated_at: timestamp,
        deleted_at: None,
    };
    let request = AppRequest::CreatePrincipal(principal);

    let lookup_key = match req.0.credential_kind {
        PrincipalCredentialKind::ApiKey => {
            let token = format!("pk_{}", Alphanumeric.sample_string(&mut rand::rng(), 32));
            sha256_hex(token.as_bytes())
        }
        _ => {
            return ApiResponse::<()>::error(ErrorStatus::Unimplemented.into());
        }
    };

    let secret_material = match req.0.credential_kind {
        PrincipalCredentialKind::ApiKey => "{}".to_owned(),
        _ => {
            return ApiResponse::<()>::error(ErrorStatus::Unimplemented.into());
        }
    };

    let principal_credential = CreatePrincipalCredential {
        credential_id: Uuid::now_v7().to_string(),
        principal_id: principal_id,
        kind: req.0.credential_kind,
        lookup_key: lookup_key,
        secret_material,
        status: PrincipalStatus::Active,
        expires_at: req.0.expires_at,
        last_used_at: None,
        created_at: timestamp,
        revoked_at: None,
    };
    let principal_credential_request = AppRequest::CreatePrincipalCredential(principal_credential);

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
