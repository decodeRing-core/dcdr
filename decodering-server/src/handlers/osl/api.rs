use actix_web::web::Data;
use actix_web::{Responder, web};
use decodering_core::actions::create_secret_mapping::CreateSecretMapping;
use decodering_core::actions::delete_secret_mapping::DeleteSecretMapping;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::repository::{AppRepository, SecretMappingRespository};
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::time::now_ts;
use decodering_core::tx::{Database, Tx};

use crate::app_data::AppData;
use crate::extractor::AuthOSLMiddleware;
use crate::handlers::osl::payload::{DeleteSecretRequestData, PutSecretRequestData};
use crate::handlers::osl::payload::{GetSecretRequestData, ListSecretRequestData};
use crate::handlers::osl::response::{
    ApiDestroySecretResponse, ApiGetSecretResponse, ApiListSecretResponse, ApiPutSecretResponse,
};
use crate::handlers::response::{ApiResponse, ErrorStatus};

pub async fn api_put_secret<D: Database + 'static>(
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<PutSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    if let Some(raft_bits) = &app.raft {
        let is_initialized = raft_bits.raft.is_initialized().await;
        if !matches!(is_initialized, Ok(true)) {
            return ApiResponse::error(ErrorStatus::NotInitialized);
        }
        if !raft_bits.raft.is_leader() {
            return ApiResponse::error(ErrorStatus::NotLeader);
        }
    }

    tracing::debug!(
        user_id = auth.user.map(|u| u.id),
        principal_id = auth.principal.map(|p| p.principal_id),
        "OSL put secret"
    );

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal);
    };

    let application = match db.app().get_by_app_id(&req.app_id).await {
        Ok(Some(app)) => app,
        Ok(None) => {
            tracing::error!("Application not found {}", req.app_id);
            return ApiResponse::error(ErrorStatus::Internal);
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::Internal);
        }
    };

    if req.options.create_only
        && let Ok(Some(_)) = db
            .secret_mapping()
            .get_by_app_id_and_secret_name(&req.app_id, &req.secret_name)
            .await
    {
        tracing::error!(app_id = %req.app_id, secret_name = %req.secret_name, "Secret already exists for app id");
        return ApiResponse::error(ErrorStatus::Internal);
    }

    let backend = core.get_backend(&req.store.backend_ref);
    let Ok(backend) = backend else {
        tracing::error!("Backend not found");
        return ApiResponse::error(ErrorStatus::UnsupportedBackend);
    };

    let secret_version = match backend.put(&req.store.store_path, &req.data) {
        Ok(version) => version,
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::Plugin);
        }
    };

    let timestamp = now_ts();
    let secret_mapping = CreateSecretMapping {
        app_id: application.app_id,
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
                ApiPutSecretResponse::new(resp.secret_name, secret_version)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to create app");
                ApiResponse::error(ErrorStatus::Internal)
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::Internal)
            }
        },
        Err(e) => {
            tracing::error!(?e);
            ApiResponse::error(ErrorStatus::Internal)
        }
    }
}

pub async fn api_get_secret<D: Database + 'static>(
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<GetSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    if let Some(raft_bits) = &app.raft {
        let is_initialized = raft_bits.raft.is_initialized().await;
        if !matches!(is_initialized, Ok(true)) {
            return ApiResponse::error(ErrorStatus::NotInitialized);
        }
    }

    tracing::debug!(
        user_id = auth.user.map(|u| u.id),
        principal_id = auth.principal.map(|p| p.principal_id),
        "OSL get secret"
    );

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal);
    };

    let secret_mapping = db
        .secret_mapping()
        .get_by_app_id_and_secret_name(&req.app_id, &req.secret_name)
        .await;

    let secret_mapping_data = match secret_mapping {
        Ok(Some(x)) => x,
        Ok(None) => {
            tracing::error!(
                "No secret mapping found for {} {}",
                req.app_id,
                req.secret_name
            );
            return ApiResponse::error(ErrorStatus::SecretNotFound);
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::Internal);
        }
    };

    let backend = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend) = backend else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::UnsupportedBackend);
    };
    match backend.get(
        &secret_mapping_data.mount_path,
        Some(req.version.to_string()),
    ) {
        Ok(out) => ApiGetSecretResponse::new(out.data, secret_mapping_data.backend, out.version),
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            ApiResponse::error(ErrorStatus::Plugin)
        }
    }
}

pub async fn api_destroy_secret<D: Database + 'static>(
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<DeleteSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    if let Some(raft_bits) = &app.raft {
        let is_initialized = raft_bits.raft.is_initialized().await;
        if !matches!(is_initialized, Ok(true)) {
            return ApiResponse::error(ErrorStatus::NotInitialized);
        }
        if !raft_bits.raft.is_leader() {
            return ApiResponse::error(ErrorStatus::NotLeader);
        }
    }

    tracing::debug!(
        user_id = auth.user.map(|u| u.id),
        principal_id = auth.principal.map(|p| p.principal_id),
        "OSL delete secret"
    );

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal);
    };

    let secret_mapping = db
        .secret_mapping()
        .get_by_app_id_and_secret_name(&req.app_id, &req.secret_name)
        .await;

    let secret_mapping_data = match secret_mapping {
        Ok(Some(x)) => x,
        Ok(None) => {
            tracing::error!(
                "No secret mapping found for {} {}",
                req.app_id,
                req.secret_name
            );
            return ApiResponse::error(ErrorStatus::SecretNotFound);
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::Internal);
        }
    };

    let backend = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend) = backend else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::UnsupportedBackend);
    };
    match backend.destroy(&secret_mapping_data.mount_path) {
        Ok(_) => {
            // Secret destroyed
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::Plugin);
        }
    }

    let request = DeleteSecretMapping::request(&req.app_id, &req.secret_name);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::DeleteSecretMapping(out) => ApiDestroySecretResponse::new(out),
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to create app");
                ApiResponse::error(ErrorStatus::Internal)
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::Internal)
            }
        },
        Err(e) => {
            tracing::error!(?e);
            ApiResponse::error(ErrorStatus::Internal)
        }
    }
}

pub async fn api_list_secret<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: web::Json<ListSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    if let Some(raft_bits) = &app.raft {
        let is_initialized = raft_bits.raft.is_initialized().await;
        if !matches!(is_initialized, Ok(true)) {
            return ApiResponse::error(ErrorStatus::NotInitialized);
        }
    }

    tracing::debug!(
        user_id = auth.user.map(|u| u.id),
        principal_id = auth.principal.map(|p| p.principal_id),
        "OSL list secrets"
    );

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal);
    };

    let secret_mapping = db
        .secret_mapping()
        .get_by_app_id_after(&req.app_id, req.after_secret.as_deref(), 100)
        .await;

    let secret_mapping_data = match secret_mapping {
        Ok(x) => x,
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::Internal);
        }
    };

    ApiListSecretResponse::new(secret_mapping_data)
}
