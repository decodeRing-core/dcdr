use actix_web::web::Data;
use actix_web::{Responder, web};
use decodering_core::actions::create_secret_mapping::CreateSecretMapping;
use decodering_core::actions::delete_secret_mapping::DeleteSecretMapping;
use decodering_core::actions::update_secret_mapping_taint::UpdateSecretMappingTaint;
use decodering_core::plugin::orchestrator::Orchestrator;
use decodering_core::plugin::osl_contract::SecretStatus;
use decodering_core::repository::{AppRepository, SecretMappingRespository};
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::time::now_ts;
use decodering_core::tx::{Database, Tx};

use crate::app_data::AppData;
use crate::auth::require_app_grant_for_principal;
use crate::error::ErrorReason;
use crate::extractor::AuthOSLMiddleware;
use crate::handlers::osl::payload::DestroySecretRequestData;
use crate::handlers::osl::payload::IsTaintedSecretRequestData;
use crate::handlers::osl::payload::PutSecretRequestData;
use crate::handlers::osl::payload::RestoreSecretRequestData;
use crate::handlers::osl::payload::TaintSecretRequestData;
use crate::handlers::osl::payload::UntaintSecretRequestData;
use crate::handlers::osl::payload::{DeleteSecretRequestData, DescribeSecretRequestData};
use crate::handlers::osl::payload::{GetSecretRequestData, ListSecretRequestData};
use crate::handlers::osl::response::ApiDeleteSecretResponse;
use crate::handlers::osl::response::ApiDestroySecretResponse;
use crate::handlers::osl::response::ApiGetSecretResponse;
use crate::handlers::osl::response::ApiIsTaintedSecretResponse;
use crate::handlers::osl::response::ApiListSecretResponse;
use crate::handlers::osl::response::ApiPutSecretResponse;
use crate::handlers::osl::response::ApiRestoreSecretResponse;
use crate::handlers::osl::response::ApiTaintSecretResponse;
use crate::handlers::osl::response::{ApiCapabilitiesResponse, ApiDescribeSecretResponse};
use crate::handlers::response::{ApiResponse, ErrorStatus};

#[tracing::instrument(
    skip_all,
    fields(
        user_id = auth.user.as_ref().map(|u| u.id),
        principal_id = auth.principal.as_ref().map(|p| p.principal_id.as_str()),
        app_id = %req.app_id,
    )
)]
pub async fn api_put_secret<D: Database + 'static>(
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<PutSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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

    let secret_version = match backend_entry.backend.put(&req.store.store_path, &req.data) {
        Ok(version) => version,
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
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
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<GetSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
    match backend_entry.backend.get(
        &secret_mapping_data.mount_path,
        Some(req.version.to_string()),
    ) {
        Ok(out) => {
            tracing::debug!(data=?out, "Plugin backend response");
            match out.status {
                SecretStatus::Present => {
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
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<DestroySecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
    match backend_entry
        .backend
        .destroy(&secret_mapping_data.mount_path)
    {
        Ok(_) => {
            // Secret destroyed
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
        }
    }

    let request = DeleteSecretMapping::request(&req.app_id, &req.secret_name);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::DeleteSecretMapping(out) => ApiDestroySecretResponse::new(out),
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
    app: Data<AppData<D>>,
    req: web::Json<ListSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
    app: Data<AppData<D>>,
    req: web::Json<TaintSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
        &secret_mapping_data.app_id,
        &secret_mapping_data.secret_name,
        true,
    );
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::UpdateSecretMappingTaint(out) => ApiTaintSecretResponse::new(out),
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
    app: Data<AppData<D>>,
    req: web::Json<UntaintSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
        &secret_mapping_data.app_id,
        &secret_mapping_data.secret_name,
        false,
    );
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::UpdateSecretMappingTaint(out) => ApiTaintSecretResponse::new(out),
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
    app: Data<AppData<D>>,
    req: web::Json<IsTaintedSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<DeleteSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
    match backend_entry
        .backend
        .delete(&secret_mapping_data.mount_path)
    {
        Ok(r) => {
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
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<RestoreSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
    match backend_entry
        .backend
        .restore(&secret_mapping_data.mount_path)
    {
        Ok(r) => {
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
    core: Data<Orchestrator>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let server_capabilities = core.get_server_capabilities();
    let backend_capabilities = core.get_backend_capabilities();
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
    app: Data<AppData<D>>,
    core: Data<Orchestrator>,
    req: web::Json<DescribeSecretRequestData>,
    auth: AuthOSLMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let _ = match require_app_grant_for_principal(&mut db, &app, &auth, &req.app_id).await {
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
    match backend_entry
        .backend
        .describe(&secret_mapping_data.mount_path)
    {
        Ok(r) => {
            return ApiDescribeSecretResponse::new(r);
        }
        Err(e) => {
            tracing::debug!(error=?e, "Plugin error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Plugin));
        }
    }
}
