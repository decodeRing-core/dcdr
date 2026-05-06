use actix_web::Responder;
use actix_web::web::{Data, Json};
use decodering_core::actions::create_app::CreateApp;
use decodering_core::actions::create_app_user::CreateAppUser;
use decodering_core::actions::create_principal::CreatePrincipal;
use decodering_core::actions::create_principal_credential::CreatePrincipalCredential;
use decodering_core::actions::create_principal_token::CreatePrincipalToken;
use decodering_core::domain::{PrincipalCredentialKind, PrincipalStatus};
use decodering_core::repository::{AppRepository, PrincipalRepository};
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::tx::{Database, Tx};
use decodering_core::{now_ts, now_ts_plus, sha256_hex};
use rand::distr::{Alphanumeric, SampleString};
use uuid::Uuid;

use crate::app_data::AppData;
use crate::extractor::AuthMiddleware;
use crate::handlers::app::payload::{AuthUserData, CreateAppData, CreateAppUserData};
use crate::handlers::app::response::{
    ApiAuthAppUserResponse, ApiCreateAppResponse, ApiCreateAppUserResponse,
};
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

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal.into());
    };

    let application = match db.app().get_by_app_id(&req.app_id).await {
        Ok(Some(app)) => app,
        Ok(None) => {
            tracing::error!("Application not found {}", req.app_id);
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    };

    let timestamp = now_ts();
    let principal_id = Uuid::now_v7().to_string();
    let principal = CreatePrincipal {
        principal_id: principal_id.clone(),
        name: req.0.name,
        app_id: application.app_id,
        kind: req.0.kind,
        status: PrincipalStatus::Active,
        created_at: timestamp,
        updated_at: timestamp,
        deleted_at: None,
    };

    let (token, lookup_key) = match req.0.credential_kind {
        PrincipalCredentialKind::ApiKey => {
            let token = format!("pk_{}", Alphanumeric.sample_string(&mut rand::rng(), 32));
            let lookup_key = sha256_hex(token.as_bytes());
            (token, lookup_key)
        }
        _ => {
            return ApiResponse::error(ErrorStatus::Unimplemented.into());
        }
    };

    let secret_material = match req.0.credential_kind {
        PrincipalCredentialKind::ApiKey => "{}".to_owned(),
        _ => {
            return ApiResponse::error(ErrorStatus::Unimplemented.into());
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

    let request = CreateAppUser::request(auth.user.id, principal, principal_credential);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreateAppUser(_) => ApiCreateAppUserResponse::new(token),
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

pub(crate) async fn auth_app_user<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<AuthUserData>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal.into());
    };

    let key_hash = sha256_hex(req.key.as_bytes());
    let principal = match db
        .principal()
        .get_by_app_id_and_key(&req.app_id, &key_hash, PrincipalStatus::Active)
        .await
    {
        Ok(Some(app)) => app,
        Ok(None) => {
            tracing::error!(
                "Principal not found {} with lookup key {}",
                req.app_id,
                key_hash
            );
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    };

    let token = format!("pk_{}", Alphanumeric.sample_string(&mut rand::rng(), 32));
    let token_hash = sha256_hex(token.as_bytes());

    let timestamp = now_ts();
    let expires = now_ts_plus(3600);
    let principal_token = CreatePrincipalToken {
        token_id: Uuid::now_v7().to_string(),
        token_hash: token_hash,
        principal_id: principal.principal_id,
        credential_id: principal.credential_id,
        issued_at: timestamp,
        expires_at: expires,
        revoked_at: None,
    };

    let request = AppRequest::CreatePrincipalToken(principal_token);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreatePrincipalToken(r) => {
                ApiAuthAppUserResponse::new(token, r.expires_at)
            }
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
