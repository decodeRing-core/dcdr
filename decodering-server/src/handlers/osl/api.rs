use std::collections::BTreeMap;
use std::sync::Arc;

use actix_web::Responder;
use actix_web::dev::ConnectionInfo;
use actix_web::web;
use actix_web::web::Data;
use decodering_core::actions::create_secret_mapping::CreateSecretMapping;
use decodering_core::actions::delete_secret_mapping::DeleteSecretMapping;
use decodering_core::actions::update_secret_mapping_taint::UpdateSecretMappingTaint;
use decodering_core::crypto::sha256_hex;
use decodering_core::metrics::Metrics;
use decodering_core::operation::OslOp;
use decodering_core::operation::OslOperation;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::plugin::osl_contract::SecretStatus;
use decodering_core::repository::AppRepository;
use decodering_core::repository::PrincipalAppGrantRepository;
use decodering_core::repository::SecretMappingRespository;
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::time::now_ts;
use decodering_core::tx::{Database, Tx};
use zeroize::Zeroizing;

use crate::app_data::AppData;
use crate::auth::require_app_grant_for_principal;
use crate::error::ErrorReason;
use crate::extractor::AuthOSLMiddleware;
use crate::handlers::osl::payload::IsTaintedSecretRequestData;
use crate::handlers::osl::payload::PutSecretRequestData;
use crate::handlers::osl::payload::RestoreSecretRequestData;
use crate::handlers::osl::payload::TaintSecretRequestData;
use crate::handlers::osl::payload::UntaintSecretRequestData;
use crate::handlers::osl::payload::{DeleteSecretRequestData, DescribeSecretRequestData};
use crate::handlers::osl::payload::{DestroySecretRequestData, ListAppsData};
use crate::handlers::osl::payload::{GetSecretRequestData, ListSecretRequestData};
use crate::handlers::osl::response::ApiGetSecretResponse;
use crate::handlers::osl::response::ApiIsTaintedSecretResponse;
use crate::handlers::osl::response::ApiListSecretResponse;
use crate::handlers::osl::response::ApiPutSecretResponse;
use crate::handlers::osl::response::ApiRestoreSecretResponse;
use crate::handlers::osl::response::ApiTaintSecretResponse;
use crate::handlers::osl::response::{ApiCapabilitiesResponse, ApiDescribeSecretResponse};
use crate::handlers::osl::response::{ApiDeleteSecretResponse, ApiListAppsResponse};
use crate::handlers::osl::response::{ApiDestroySecretResponse, ApiListBackendsResponse};
use crate::handlers::response::{ApiResponse, ErrorStatus};
use crate::plugin::get_plugin_config_credentials_for_backend;

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_put_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<PutSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Put);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
    };

    let application = match db.app().get_by_app_id(&req.app_id).await {
        Ok(Some(app)) => app,
        Ok(None) => {
            tracing::error!("Application not found {}", req.app_id);
            return ApiResponse::error(ErrorStatus::OperationFailed(
                ErrorReason::ApplicationNotFound,
            ));
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    if req.options.create_only
        && let Ok(Some(_)) = db
            .secret_mapping()
            .get_by_app_id_and_secret_name(&req.app_id, &req.secret_name)
            .await
    {
        tracing::error!(app_id = %req.app_id, secret_name = %req.secret_name, "Secret already exists for app id");
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::SecretAlreadyExists,
        ));
    }

    let backend_entry = core.get_backend(&req.store.backend_ref);
    let Ok(backend_entry) = backend_entry else {
        tracing::error!("Backend not found");
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::UnsupportedBackend,
        ));
    };

    let credentials = match get_plugin_config_credentials_for_backend(
        &mut db,
        &app,
        &req.store.backend_ref,
    )
    .await
    {
        Ok(x) => x,
        Err(err) => return ApiResponse::error(err),
    };

    // Idempotency token if required by the vault
    let token = if let Some(x) = &req.idempotency_token {
        x.clone()
    } else {
        let basis = if req.options.create_only {
            format!("create:{}:{}", req.app_id, req.secret_name)
        } else {
            let sorted: BTreeMap<String, serde_json::Value> =
                serde_json::from_value(req.data.clone()).unwrap_or_default();
            let canonical_json = serde_json::to_string(&sorted).unwrap_or_default();
            let data_hash = sha256_hex(canonical_json.as_bytes());
            format!("put:{}:{}:{}", req.app_id, req.secret_name, data_hash)
        };
        sha256_hex(basis.as_bytes())
    };

    let secret_version =
        match backend_entry
            .backend
            .put(&req.store.store_path, &req.data, &token, &credentials)
        {
            Ok(version) => version,
            Err(e) => {
                tracing::debug!(error=?e, "Plugin error");
                return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
            }
        };

    let timestamp = now_ts();
    let secret_mapping = CreateSecretMapping {
        actor: auth.actor(&conn),
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
                op.ok();
                ApiPutSecretResponse::new(resp.secret_name, secret_version)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to put secret");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::GenericFail(
                    "put secret".into(),
                )))
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
            }
        },
        Err(e) => {
            tracing::error!(?e);
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Internal))
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_get_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<GetSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Get);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
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
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    if secret_mapping_data.tainted == 1 {
        tracing::debug!("Secret is tainted and is currently inaccessible");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::TaintedSecret));
    }

    let backend_entry = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend_entry) = backend_entry else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::UnsupportedBackend,
        ));
    };

    let credentials = match get_plugin_config_credentials_for_backend(
        &mut db,
        &app,
        &secret_mapping_data.backend,
    )
    .await
    {
        Ok(x) => x,
        Err(err) => return ApiResponse::error(err),
    };

    match backend_entry.backend.get(
        &secret_mapping_data.mount_path,
        req.version.clone(),
        &credentials,
    ) {
        Ok(out) => {
            tracing::debug!(data=?out, "Plugin backend response");
            match out.status {
                SecretStatus::Present => {
                    op.ok();
                    return ApiGetSecretResponse::new(
                        out.data.unwrap_or_default(),
                        secret_mapping_data.backend,
                        out.version,
                    );
                }
                SecretStatus::Destroyed | SecretStatus::Disabled => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Secret(
                        "Secret is no longer available for retrieval",
                    )));
                }
                SecretStatus::NotFound => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(
                        ErrorReason::SecretNotFound,
                    ));
                }
                SecretStatus::SoftDeleted => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Secret(
                        "Secret has been soft deleted",
                    )));
                }
            }
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin))
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_destroy_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<DestroySecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Destroy);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
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
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let backend_entry = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend_entry) = backend_entry else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::UnsupportedBackend,
        ));
    };

    let credentials = match get_plugin_config_credentials_for_backend(
        &mut db,
        &app,
        &secret_mapping_data.backend,
    )
    .await
    {
        Ok(x) => x,
        Err(err) => return ApiResponse::error(err),
    };

    match backend_entry
        .backend
        .destroy(&secret_mapping_data.mount_path, &credentials)
    {
        Ok(_) => {
            // Secret destroyed
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
        }
    }

    let request = DeleteSecretMapping::request(auth.actor(&conn), &req.app_id, &req.secret_name);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::DeleteSecretMapping(out) => {
                op.ok();
                ApiDestroySecretResponse::new(out)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to destroy secret");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::GenericFail(
                    "destroy secret".into(),
                )))
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
            }
        },
        Err(e) => {
            tracing::error!(?e);
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Internal))
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_list_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: web::Json<ListSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::List);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
    };

    let secret_mapping = db
        .secret_mapping()
        .get_by_app_id_after(&req.app_id, req.after_secret.as_deref(), 100)
        .await;

    let secret_mapping_data = match secret_mapping {
        Ok(x) => x,
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    op.ok();
    ApiListSecretResponse::new(secret_mapping_data)
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_taint_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: web::Json<TaintSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Taint);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
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
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let request = UpdateSecretMappingTaint::request(
        auth.actor(&conn),
        &secret_mapping_data.app_id,
        &secret_mapping_data.secret_name,
        true,
    );
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::UpdateSecretMappingTaint(out) => {
                op.ok();
                ApiTaintSecretResponse::new(out)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to taint secret");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::GenericFail(
                    "taint secret".into(),
                )))
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
            }
        },
        Err(e) => {
            tracing::error!(?e);
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Internal))
        }
    }
}

pub async fn api_untaint_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: web::Json<UntaintSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Untaint);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
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
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let request = UpdateSecretMappingTaint::request(
        auth.actor(&conn),
        &secret_mapping_data.app_id,
        &secret_mapping_data.secret_name,
        false,
    );
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::UpdateSecretMappingTaint(out) => {
                op.ok();
                ApiTaintSecretResponse::new(out)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to untaint secret");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::GenericFail(
                    "untaint secret".into(),
                )))
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
            }
        },
        Err(e) => {
            tracing::error!(?e);
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Internal))
        }
    }
}

pub async fn api_is_tainted_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: web::Json<IsTaintedSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::IsTaint);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
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
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    op.ok();
    ApiIsTaintedSecretResponse::new(secret_mapping_data.tainted == 1)
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_delete_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<DeleteSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Delete);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
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
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let backend_entry = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend_entry) = backend_entry else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::UnsupportedBackend,
        ));
    };

    let credentials = match get_plugin_config_credentials_for_backend(
        &mut db,
        &app,
        &secret_mapping_data.mount_path,
    )
    .await
    {
        Ok(x) => x,
        Err(err) => return ApiResponse::error(err),
    };

    match backend_entry
        .backend
        .delete(&secret_mapping_data.mount_path, &credentials)
    {
        Ok(r) => {
            op.ok();
            // Secret soft deleted
            return ApiDeleteSecretResponse::new(r);
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_restore_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<RestoreSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Restore);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
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
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let backend_entry = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend_entry) = backend_entry else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::UnsupportedBackend,
        ));
    };
    let credentials = match get_plugin_config_credentials_for_backend(
        &mut db,
        &app,
        &secret_mapping_data.mount_path,
    )
    .await
    {
        Ok(x) => x,
        Err(err) => return ApiResponse::error(err),
    };
    match backend_entry
        .backend
        .restore(&secret_mapping_data.mount_path, &credentials)
    {
        Ok(r) => {
            op.ok();
            // Secret soft delete restore
            return ApiRestoreSecretResponse::new(r);
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
    )
)]
pub async fn api_get_capabilities<D: Database + 'static>(
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Capabilities);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };
    let mut credentials: BTreeMap<String, BTreeMap<String, Zeroizing<String>>> = BTreeMap::new();
    for backend in core.get_backends().keys() {
        let plugin_credential = get_plugin_config_credentials_for_backend(&mut db, &app, backend)
            .await
            .unwrap_or_default();
        credentials.insert(backend.clone(), plugin_credential);
    }
    let server_capabilities = core.get_server_capabilities(&credentials);
    let backend_capabilities = core.get_backend_capabilities(&credentials);
    op.ok();
    return ApiCapabilitiesResponse::new(server_capabilities, backend_capabilities);
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_describe_secret<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<DescribeSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::Describe);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&conn, &mut db, &app, &auth, &req.app_id).await {
        Ok(grant) => grant,
        Err(err) => return ApiResponse::error(err),
    };

    let secret_mapping = db
        .secret_mapping()
        .get_by_app_id_and_secret_name(&req.app_id, &req.secret_name)
        .await;

    let secret_mapping_data = match secret_mapping {
        Ok(Some(x)) => x,
        Ok(None) => {
            tracing::error!("Secret not found");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::SecretNotFound));
        }
        Err(e) => {
            tracing::error!(%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let backend_entry = core.get_backend(&secret_mapping_data.backend);
    let Ok(backend_entry) = backend_entry else {
        tracing::error!("Backend not found {}", secret_mapping_data.backend);
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::UnsupportedBackend,
        ));
    };

    let credentials = match get_plugin_config_credentials_for_backend(
        &mut db,
        &app,
        &secret_mapping_data.mount_path,
    )
    .await
    {
        Ok(x) => x,
        Err(err) => return ApiResponse::error(err),
    };

    match backend_entry
        .backend
        .describe(&secret_mapping_data.mount_path, &credentials)
    {
        Ok(r) => {
            op.ok();
            return ApiDescribeSecretResponse::new(r);
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
        }
    }
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
    )
)]
pub async fn api_get_apps_list<D: Database + 'static>(
    app: Data<AppData<D>>,
    auth: AuthOSLMiddleware<D>,
    params: web::Query<ListAppsData>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::ListApps);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let principal = auth.principal;
    let Some(principal) = principal else {
        tracing::error!("Principal not found");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
    };

    let principal_app_grants = match db
        .principal_app_grant()
        .get_by_principal_id_after(&principal.principal_id, params.after_app_id.as_deref(), 64)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };
    op.ok();
    ApiListAppsResponse::new(principal_app_grants)
}

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
    )
)]
pub async fn api_get_backends_list<D: Database + 'static>(
    _app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    auth: AuthOSLMiddleware<D>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut op = OslOp::start(metrics.get_ref().clone(), OslOperation::ListBackends);
    let backends = core.get_backends();

    let backend_names: Vec<String> = backends.iter().map(|f| f.0.to_owned()).collect();
    op.ok();
    ApiListBackendsResponse::new(backend_names)
}
