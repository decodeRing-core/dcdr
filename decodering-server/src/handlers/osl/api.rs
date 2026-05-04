use actix_web::web::Data;
use actix_web::{Responder, web};
use decodering_core::actions::create_secret_mapping::CreateSecretMapping;
use decodering_core::now_ts;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::repository::SecretMappingRespository;
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::tx::{Database, Tx};

use crate::app_data::AppData;
use crate::handlers::osl::payload::GetSecretRequestData;
use crate::handlers::osl::payload::PutSecretRequestData;
use crate::handlers::osl::response::{ApiGetSecretResponse, ApiPutSecretResponse};
use crate::handlers::response::{ApiResponse, ErrorStatus};

pub(crate) async fn api_put_secret<D: Database + 'static>(
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<PutSecretRequestData>,
) -> impl Responder {
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

    // Do we have an app_id? Is the token valid and is the user admin ?
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal.into());
    };

    let backend = core.get_backend(&req.store.backend_ref);
    let Ok(backend) = backend else {
        tracing::error!("Backend not found");
        return ApiResponse::error(ErrorStatus::UnsupportedBackend);
    };

    let secret_version = match backend.put(&req.store.store_path, &req.data) {
        Ok(version) => version,
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::Plugin.into());
        }
    };

    let timestamp = now_ts();
    let secret_mapping = CreateSecretMapping {
        app_id: req.app_id.clone(),
        secret_name: req.secret_name.clone(),
        backend: req.store.backend_ref.clone(),
        mount_path: req.store.store_path.clone(),
        tainted: 0,
        created_at: timestamp,
        updated_at: timestamp,
    };
    let request = AppRequest::CreateSecretMapping(secret_mapping);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreateSecretMapping(resp) => {
                return ApiPutSecretResponse::new(resp.secret_name, secret_version.to_string());
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

pub(crate) async fn api_get_secret<D: Database + 'static>(
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<GetSecretRequestData>,
) -> impl Responder {
    match &app.raft {
        Some(raft_bits) => {
            let is_initialized = raft_bits.raft.is_initialized().await;
            if !matches!(is_initialized, Ok(true)) {
                return ApiResponse::error(ErrorStatus::NotInitialized.into());
            }
        }
        _ => {}
    }

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal.into());
    };

    let secret_mapping = db
        .secret_mapping()
        .get_by_app_id_secret_name(&req.app_id, &req.secret_name)
        .await;

    let secret_mapping_data = match secret_mapping {
        Ok(Some(x)) => x,
        Ok(None) => {
            tracing::error!(
                "No secret mapping found for {}/{}",
                req.app_id,
                req.secret_name
            );
            return ApiResponse::error(ErrorStatus::SecretNotFound.into());
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    };

    let backend = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend) = backend else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::UnsupportedBackend);
    };
    let out = backend.get(&req.secret_name, Some(req.version.to_string()));
    let Ok(out) = out else {
        let e = out.unwrap_err();
        tracing::debug!(error=?e, "Plugin error");
        return ApiResponse::error(ErrorStatus::Plugin.into());
    };

    return ApiGetSecretResponse::new(out.data);
}
