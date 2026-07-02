use std::sync::Arc;

use crate::app_data::AppData;
use crate::audit::{unlock_audit_allowed, unlock_audit_errored};
use crate::error::AppError::{Action, Raft};
use crate::error::ErrorReason;
use crate::extractor::AuthAdminMiddleware;
use crate::handlers::response::{ApiResponse, ApiStatus, ErrorStatus, SuccessStatus};
use crate::handlers::system::payloads::{InitSystemData, PluginConfigData, UnlockData};
use crate::handlers::system::response::{ApiInitSystemResponse, ApiSystemStatusResponse};
use actix_web::Responder;
use actix_web::dev::ConnectionInfo;
use actix_web::web::{Data, Json};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use decodering_core::actions::create_api_key::CreateApiKey;
use decodering_core::actions::create_plugin_config::CreatePluginConfig;
use decodering_core::actions::create_shamir_configuration::CreateShamirConfiguration;
use decodering_core::actions::create_user::CreateUser;
use decodering_core::actions::system_init::SystemInit;
use decodering_core::actions::update_plugin_config_credentials::UpdatePluginConfigCredentials;
use decodering_core::audit::Actor;
use decodering_core::crypto::{encrypt_map, sha256_hex};
use decodering_core::metrics::Metrics;
use decodering_core::metrics::unlock_attempt::UnlockAttempt;
use decodering_core::repository::ShamirRepository;
use decodering_core::response::AppResponse;
use decodering_core::shamir::{initialize_shamir, unlock};
use decodering_core::time::now_ts;
use decodering_core::tx::{Database, Tx};
use rand::distr::{Alphanumeric, SampleString};
use zeroize::Zeroizing;

pub async fn system_init<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: Json<InitSystemData>,
) -> impl Responder {
    let ip = conn.peer_addr().map(str::to_owned);
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };
    let shamir_configuration = db.shamir().get_first().await;
    let total_shares = match shamir_configuration {
        Ok(Some(_)) => 1,
        Ok(None) => 0,
        Err(e) => {
            tracing::error!(error=%e, "Database error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let shamir_setup = if total_shares == 0 {
        tracing::debug!("Initializing shamir secret sharing");

        let Some(total_shares) = req.total_shares else {
            tracing::error!("Shard data initialization required. Missing total shares.");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::MissingData(
                "total shares",
            )));
        };
        let Some(threshold) = req.threshold else {
            tracing::error!("Shard data initialization required. Missing threshold.");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::MissingData(
                "threshold",
            )));
        };

        let shamir_init = match initialize_shamir(total_shares, threshold) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error=%e, "Shamir secret sharing initialization failed");
                return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Internal));
            }
        };
        Some((shamir_init, total_shares, threshold))
    } else {
        None
    };

    let Some((shamir_init, total_shares, threshold)) = shamir_setup else {
        tracing::error!("System already initialized");
        return ApiResponse::error(ErrorStatus::OperationFailed(
            ErrorReason::AlreadyInitialized,
        ));
    };

    let token = format!("pk_{}", Alphanumeric.sample_string(&mut rand::rng(), 32));
    let token_prefix: String = token.chars().take(8).collect();
    let token_hash = sha256_hex(token.as_bytes());
    let shamir = CreateShamirConfiguration::new(
        Actor::unauthenticated(ip.clone()),
        i16::from(total_shares),
        i16::from(threshold),
        shamir_init.hash,
    );
    let user = CreateUser::new(
        Actor::unauthenticated(ip.clone()),
        "root",
        "root@localhost",
        "",
        1,
    );
    let api_key = CreateApiKey::init(
        Actor::unauthenticated(ip.clone()),
        token_hash,
        token_prefix,
        None,
    );
    let mut plugins: Vec<CreatePluginConfig> = vec![];
    let timestamp = now_ts();
    let key = shamir_init.master_key;
    for (backend_name, credentials) in req.plugins_credentials.clone() {
        let blob = encrypt_map(&key, &credentials, backend_name.as_bytes());
        if let Ok(blob) = blob {
            let plugin_config = CreatePluginConfig::new(
                Actor::unauthenticated(ip.clone()),
                backend_name,
                blob,
                timestamp,
            );
            plugins.push(plugin_config);
        }
    }

    let request_initialize = SystemInit::request(
        Actor::unauthenticated(ip.clone()),
        shamir,
        user,
        api_key,
        plugins,
    );
    match app.submit(request_initialize).await {
        Ok(resp) => match resp {
            AppResponse::SystemInit(_) => {
                let shards: Vec<String> = shamir_init
                    .shards
                    .iter()
                    .map(|sh| STANDARD.encode(Vec::from(sh)))
                    .collect();
                ApiInitSystemResponse::initialized(shards, Some(token))
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to initialize system");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to initialize system".into(),
                )))
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
            }
        },
        Err(e) => {
            tracing::error!(?e);
            match e {
                Action(action_error) => ApiResponse::error(ErrorStatus::OperationFailed(
                    ErrorReason::Message(action_error.to_string().into()),
                )),
                Raft(raft_error) => ApiResponse::error(ErrorStatus::OperationFailed(
                    ErrorReason::Message(raft_error.to_string().into()),
                )),
            }
        }
    }
}

pub async fn system_unlock<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: Json<UnlockData>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut attempt = UnlockAttempt::start(metrics.get_ref().clone());
    let ip = conn.peer_addr().map(str::to_owned);
    if app.master_key.get().is_some() {
        attempt.ok();
        tracing::info!("Node already unlocked");
        return ApiResponse::empty(SuccessStatus::SystemUnlocked.into());
    }
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to DB");
        return ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };
    let shamir_configuration = match db.shamir().get_first().await {
        Ok(Some(config)) => config,
        Ok(None) => {
            attempt.denied();
            return ApiResponse::error(ErrorStatus::OperationFailed(
                ErrorReason::SystemNotInitialized,
            ));
        }
        Err(e) => {
            tracing::error!(error=%e, "Failed to unlock node");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let threshold = u8::try_from(shamir_configuration.threshold);
    let Ok(threshold) = threshold else {
        attempt.denied();
        tracing::error!("Shamir configuration threshold out of range");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::InvalidShamirKeys));
    };

    let secret = unlock(
        threshold,
        &shamir_configuration.validation_hash,
        &req.shards,
    );
    match secret {
        Ok(secret) => {
            let out = app.master_key.set(Zeroizing::new(secret));
            match out {
                Ok(()) => {
                    unlock_audit_allowed::<D>(ip, db).await;
                    attempt.ok();
                    ApiResponse::empty(SuccessStatus::SystemUnlocked.into())
                }
                Err(e) => {
                    unlock_audit_errored::<D>(ip, db, "Invalid shards".to_owned()).await;
                    tracing::error!(err=?e, "Unlock error");
                    attempt.denied();
                    ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Internal))
                }
            }
        }
        Err(err) => {
            unlock_audit_errored::<D>(ip, db, err.to_string()).await;
            tracing::error!(e=%err, "Failed to unlock system");
            attempt.denied();
            ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                "Failed to unlock system".into(),
            )))
        }
    }
}

pub async fn system_status<D: Database + 'static>(app: Data<AppData<D>>) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to DB");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let system_initialized = match db.shamir().get_first().await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            tracing::error!(error=%e, "System status error");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let system_unlocked = app.master_key.get().is_some();

    ApiSystemStatusResponse::new(system_initialized, system_unlocked)
}

pub async fn system_plugin_config<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: Json<PluginConfigData>,
    auth: AuthAdminMiddleware<D>,
) -> impl Responder {
    let timestamp = now_ts();
    let Some(key) = app.master_key.get() else {
        tracing::error!("System is locked");
        return ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::Locked));
    };

    for (backend_name, credentials) in req.plugins_credentials.clone() {
        let blob = encrypt_map(key, &credentials, backend_name.as_bytes());
        if let Ok(blob) = blob {
            let request_update_plugin = UpdatePluginConfigCredentials::request(
                auth.actor(&conn),
                backend_name,
                blob,
                timestamp,
            );

            match app.submit(request_update_plugin).await {
                Ok(resp) => match resp {
                    AppResponse::UpdatePluginConfigSecrets(_) => ApiResponse::<()>::new(
                        ApiStatus::Success(SuccessStatus::OperationCompleted),
                        None,
                    ),
                    AppResponse::Error(e) => {
                        tracing::error!(%e, "Failed to initialize system");
                        ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                            "Failed to initialize system".into(),
                        )))
                    }
                    other_api_response => {
                        tracing::error!(?other_api_response, "unexpected AppResponse variant");
                        ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
                    }
                },
                Err(e) => {
                    tracing::error!(%e, "Failed to initialize system");
                    match e {
                        Action(action_error) => {
                            return ApiResponse::error(ErrorStatus::OperationFailed(
                                ErrorReason::Message(action_error.to_string().into()),
                            ));
                        }
                        Raft(raft_error) => {
                            return ApiResponse::error(ErrorStatus::OperationFailed(
                                ErrorReason::Message(raft_error.to_string().into()),
                            ));
                        }
                    }
                }
            };
        }
    }
    ApiResponse::new(ApiStatus::Success(SuccessStatus::OperationCompleted), None)
}
