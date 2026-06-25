use std::str::FromStr;
use std::sync::Arc;

use actix_web::Responder;
use actix_web::dev::ConnectionInfo;
use actix_web::web::{Data, Json};
use decodering_core::actions::create_app::CreateApp;
use decodering_core::actions::create_app_user::CreateAppUser;
use decodering_core::actions::create_auth_challenge::CreateAuthChallenge;
use decodering_core::actions::create_principal::CreatePrincipal;
use decodering_core::actions::create_principal_app_grant::CreatePrincipalAppGrant;
use decodering_core::actions::create_principal_app_grant::CreatePrincipalAppGrants;
use decodering_core::actions::create_principal_credential::CreatePrincipalCredential;
use decodering_core::actions::create_principal_token::CreatePrincipalToken;
use decodering_core::actions::delete_principal_app_grant::DeletePrincipalAppGrant;
use decodering_core::actions::update_auth_challenge_consumed_at::UpdateAuthChallengeConsumedAt;
use decodering_core::actions::update_principal_credential_status::UpdatePrincipalCredentialStatus;
use decodering_core::audit::Actor;
use decodering_core::auth::registry::AuthRegistry;
use decodering_core::auth::types::ActivateRequest;
use decodering_core::auth::types::AuthRequest;
use decodering_core::auth::types::ChallengeRequest;
use decodering_core::auth::types::EnrollRequest;
use decodering_core::auth::types::ResolveRequest;
use decodering_core::crypto::sha256_hex;
use decodering_core::domain::{PrincipalCredentialKind, PrincipalStatus};
use decodering_core::metrics::Metrics;
use decodering_core::metrics::app_auth_attempt::{AppAuthAttempt, AppAuthAttemptMethod};
use decodering_core::repository::AppRepository;
use decodering_core::repository::PrincipalRepository;
use decodering_core::repository::{PrincipalAppGrantRepository, PrincipalCredentialRepository};
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::time::{CHALLENGE_TTL_SECS, now_ts, now_ts_plus};
use decodering_core::tx::{Database, Tx};
use rand::Rng;
use rand::distr::{Alphanumeric, SampleString};
use serde_json::json;
use uuid::Uuid;

use crate::app_data::AppData;
use crate::config::Config;
use crate::error::ErrorReason;
use crate::extractor::AuthAdminMiddleware;
use crate::handlers::app::payload::CreateAppUserData;
use crate::handlers::app::payload::RevokeAppData;
use crate::handlers::app::payload::{AppGrantData, AuthChallengeData};
use crate::handlers::app::payload::{AuthActivationData, CreateAppData};
use crate::handlers::app::payload::{AuthUserData, ListAppsData};
use crate::handlers::app::response::ApiAuthAppUserResponse;
use crate::handlers::app::response::ApiAuthChallengeResponse;
use crate::handlers::app::response::ApiCreateAppGrantResponse;
use crate::handlers::app::response::ApiDeleteAppGrantResponse;
use crate::handlers::app::response::{ApiCreateAppResponse, ApiCreateAppUserResponse};
use crate::handlers::osl::response::ApiListAppsResponse;
use crate::handlers::response::{ApiResponse, ApiStatus, ErrorStatus, SuccessStatus};

pub async fn create_app_user<D: Database + 'static>(
    conn: ConnectionInfo,
    config: Data<Config>,
    app: Data<AppData<D>>,
    req: Json<CreateAppUserData>,
    auth: AuthAdminMiddleware<D>,
    registry: Data<AuthRegistry>,
) -> impl Responder {
    let timestamp = now_ts();
    let principal_id = Uuid::now_v7().to_string();
    let principal = CreatePrincipal {
        actor: auth.actor(&conn),
        principal_id: principal_id.clone(),
        name: req.0.name,
        kind: req.0.kind,
        status: PrincipalStatus::Active,
        created_at: timestamp,
        updated_at: timestamp,
        deleted_at: None,
    };

    let auth_method = registry.get(req.0.credential_kind.as_str());
    let Some(auth_method) = auth_method else {
        tracing::error!("Unsupported auth method");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::UnsupportedAuth));
    };

    let auth_resp = match auth_method
        .enroll(EnrollRequest {
            principal_id: principal_id.clone(),
            data: req.0.data.unwrap_or_default(),
            now: timestamp,
            config: json!({"tpm_trust_dir": config.tpm_trust_dir}),
        })
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Auth error: {e:?}");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
        }
    };

    let principal_credential = CreatePrincipalCredential {
        actor: auth.actor(&conn),
        credential_id: Uuid::now_v7().to_string(),
        principal_id: principal_id.clone(),
        kind: req.0.credential_kind,
        lookup_key: auth_resp.lookup_key,
        secret_material: auth_resp.secret_material.to_string(),
        status: auth_resp.status,
        expires_at: req.0.expires_at,
        last_used_at: None,
        created_at: timestamp,
        revoked_at: None,
    };

    let mut principal_app_grants = vec![];
    for app_id in req.0.apps.unwrap_or_default() {
        let app_grant = CreatePrincipalAppGrant {
            actor: auth.actor(&conn),
            principal_id: principal_id.clone(),
            app_id: app_id.clone(),
            granted_at: timestamp,
            granted_by: Some(auth.user.id),
            revoked_at: None,
            revoked_by: None,
        };
        principal_app_grants.push(app_grant);
    }
    let request = CreateAppUser::request(
        auth.actor(&conn),
        principal,
        principal_credential,
        principal_app_grants,
    );
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreateAppUser(resp) => {
                let credential_id = resp.principal_credential.credential_id;
                ApiCreateAppUserResponse::new(auth_resp.client_payload, principal_id, credential_id)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to create app user");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to create app user".into(),
                )))
            }
            other_api_response => {
                tracing::error!(?other_api_response, "Unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
            }
        },
        Err(e) => {
            tracing::error!(?e);
            match e {
                Action(action_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        action_error.to_string().into(),
                    )));
                }
                Raft(raft_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        raft_error.to_string().into(),
                    )));
                }
            }
        }
    }
}

pub async fn create_app<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: Json<CreateAppData>,
    auth: AuthAdminMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };
    match db.app().get_by_app_name(&req.0.app_name).await {
        Ok(Some(a)) => {
            tracing::warn!(
                name = a.app_name,
                id = a.app_id,
                "Cannot create a duplicate app with name"
            );
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::DuplicatedApp));
        }
        Ok(None) => (),
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    }

    let request = CreateApp::request(
        auth.actor(&conn),
        Uuid::now_v7().to_string(),
        req.0.app_name,
    );
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreateApp(a) => ApiCreateAppResponse::new(a.app_id, a.app_name),
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to create app");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to create app".into(),
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
                Action(action_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        action_error.to_string().into(),
                    )));
                }
                Raft(raft_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        raft_error.to_string().into(),
                    )));
                }
            }
        }
    }
}

pub async fn auth_app_user<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    body: Json<AuthUserData>,
    registry: Data<AuthRegistry>,
    metrics: Data<Arc<dyn Metrics>>,
) -> impl Responder {
    let mut attempt = AppAuthAttempt::start(metrics.get_ref().clone(), AppAuthAttemptMethod::None);

    let ip = conn.peer_addr().map(str::to_owned);

    let auth_method = registry.get(&body.0.credential_kind);
    let Some(auth_method) = auth_method else {
        tracing::error!("Unsupported auth method");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::UnsupportedAuth));
    };

    attempt.method(AppAuthAttemptMethod::from(auth_method.kind()));
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let caps = auth_method.capabilities();
    let credential = if caps.requires_resolve {
        let lookup_key = auth_method
            .resolve(ResolveRequest {
                proof: body.0.proof.clone(),
                config: serde_json::Value::default(),
            })
            .await;
        let Ok(lookup_key) = lookup_key else {
            attempt.denied();
            tracing::error!("Failed to get lookup_key");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
        };
        let credential = match db
            .principal_credential()
            .get_active_by_kind_and_lookup_key(
                PrincipalCredentialKind::TrustedPlatformModule,
                &lookup_key,
            )
            .await
        {
            Ok(Some(x)) => x,
            Ok(None) => {
                attempt.denied();
                tracing::error!("Active credential not found");
                return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
            }
            Err(e) => {
                tracing::error!(err=?e, "Failed to query database");
                return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
            }
        };

        if credential.status != PrincipalStatus::Active || credential.revoked_at.is_some() {
            attempt.denied();

            tracing::error!(
                status=credential.status.as_str(),
                revoked_at=?credential.revoked_at,
                "Invalid credential"
            );
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
        }
        Some(credential)
    } else {
        None
    };

    let challenge_state = if caps.requires_challenge {
        let challenge_id =
            if let Some(x) = body.0.proof.get("challenge_id").and_then(|v| v.as_str()) {
                x.to_owned()
            } else {
                attempt.denied();
                tracing::error!("Expected challenge_id param");
                return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
            };

        let consumed_tpm_challenge = UpdateAuthChallengeConsumedAt {
            actor: Actor::unauthenticated(ip.clone()),
            challenge_id,
            consumed_at: now_ts(),
        };

        let request = AppRequest::UpdateConsumedAt(consumed_tpm_challenge);
        match app.submit(request).await {
            Ok(resp) => match resp {
                AppResponse::ConsumeAuthChallenge(resp) => Some(resp.payload),
                AppResponse::Error(e) => {
                    attempt.denied();
                    tracing::error!(%e, "Failed to consume tpm challenge");
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        "Failed to consume tpm challenge".into(),
                    )));
                }
                other_api_response => {
                    tracing::error!(?other_api_response, "unexpected AppResponse variant");
                    return ApiResponse::error(ErrorStatus::OperationFailed(
                        ErrorReason::Unexpected,
                    ));
                }
            },
            Err(e) => {
                tracing::error!(?e);
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
        }
    } else {
        None
    };

    let timestamp = now_ts();

    let credential_material =
        credential.and_then(|f| serde_json::Value::from_str(&f.secret_material).ok());
    let auth_resp = match auth_method
        .authenticate(AuthRequest {
            proof: body.proof.clone(),
            challenge_state,
            credential_material,
            now: timestamp,
            config: serde_json::Value::default(),
        })
        .await
    {
        Ok(resp) => resp,
        Err(a) => {
            attempt.denied();
            tracing::error!("Auth error: {a:?}");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
        }
    };

    let principal = match db
        .principal()
        .get_active_by_key(&auth_resp.lookup_key, PrincipalStatus::Active)
        .await
    {
        Ok(Some(app)) => app,
        Ok(None) => {
            attempt.denied();
            let metadata = auth_resp.audit_metadata;
            tracing::error!(lookup_key=%metadata,"Principal not found with lookup key");
            return ApiResponse::error(ErrorStatus::OperationFailed(
                ErrorReason::PrincipalNotFound,
            ));
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let token = format!("tok_{}", Alphanumeric.sample_string(&mut rand::rng(), 32));
    let token_hash = sha256_hex(token.as_bytes());

    let expires = now_ts_plus(3600);
    let principal_token = CreatePrincipalToken {
        actor: Actor::Principal {
            principal_id: principal.principal_id.clone(),
            ip: ip.clone(),
        },
        token_id: Uuid::now_v7().to_string(),
        token_hash,
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
                attempt.ok();
                ApiAuthAppUserResponse::new(token, r.expires_at)
            }
            AppResponse::Error(e) => {
                attempt.denied();
                tracing::error!(%e, "Failed to authenticate app user");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to authenticate app user".into(),
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
                Action(action_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        action_error.to_string().into(),
                    )));
                }
                Raft(raft_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        raft_error.to_string().into(),
                    )));
                }
            }
        }
    }
}

pub async fn auth_challenge_app_user<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    body: Json<AuthChallengeData>,
    registry: Data<AuthRegistry>,
) -> impl Responder {
    let ip = conn.peer_addr().map(str::to_owned);

    let auth_method = registry.get(&body.0.credential_kind);
    let Some(auth_method) = auth_method else {
        tracing::error!("Unsupported auth method");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::UnsupportedAuth));
    };

    let now = now_ts();
    let mut nonce_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut nonce_bytes);

    let auth_resp = match auth_method
        .challenge(ChallengeRequest {
            hint: body.0.hint,
            entropy: nonce_bytes.to_vec(),
            now,
            config: serde_json::Value::default(),
        })
        .await
    {
        Ok(resp) => resp,
        Err(a) => {
            tracing::error!("Auth error: {a:?}");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
        }
    };

    let challenge_id = Uuid::now_v7().to_string();
    let expires_at = now + CHALLENGE_TTL_SECS;

    let auth_challenge = CreateAuthChallenge {
        actor: Actor::None { ip: ip.clone() },
        challenge_id,
        method: body.0.credential_kind,
        payload: auth_resp.challenge_state,
        issued_at: now,
        expires_at,
        consumed_at: None,
    };
    let tpm_request = AppRequest::CreateAuthChallenge(auth_challenge);
    match app.submit(tpm_request).await {
        Ok(resp) => match resp {
            AppResponse::CreateAuthChallenge(r) => {
                ApiAuthChallengeResponse::new(r.challenge_id, auth_resp.client_payload, expires_at)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to generate auth challenge");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to generate auth challenge".into(),
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
                Action(action_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        action_error.to_string().into(),
                    )));
                }
                Raft(raft_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        raft_error.to_string().into(),
                    )));
                }
            }
        }
    }
}

pub async fn auth_activate_app_user<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    body: Json<AuthActivationData>,
    registry: Data<AuthRegistry>,
) -> impl Responder {
    let auth_method = registry.get(&body.0.credential_kind);
    let Some(auth_method) = auth_method else {
        tracing::error!("Unsupported auth method");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::UnsupportedAuth));
    };

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::<()>::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let credential = match db
        .principal_credential()
        .get_pending_by_kind_and_credential_and_principal(
            body.principal_id.clone(),
            body.credential_id.clone(),
            PrincipalCredentialKind::TrustedPlatformModule,
        )
        .await
    {
        Ok(Some(x)) => x,
        Ok(None) => {
            tracing::error!("Pending credential not found");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    if credential.status != PrincipalStatus::Pending || credential.revoked_at.is_some() {
        tracing::error!(
            status=credential.status.as_str(),
            revoked_at=?credential.revoked_at,
            "Invalid credential"
        );
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
    }

    let material = serde_json::from_str(&credential.secret_material);
    let Ok(material) = material else {
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
    };

    let timestamp = now_ts();
    let auth_resp = match auth_method
        .activate(ActivateRequest {
            principal_id: body.0.principal_id,
            credential_id: body.0.credential_id,
            credential_material: material,
            proof: body.0.proof,
            now: timestamp,
            config: serde_json::Value::default(),
        })
        .await
    {
        Ok(resp) => resp,
        Err(a) => {
            tracing::error!("Auth error: {a:?}");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
        }
    };

    if !auth_resp.activated {
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unauthorized));
    }

    let ip = conn.peer_addr().map(str::to_owned);
    let status_update = UpdatePrincipalCredentialStatus {
        actor: Actor::Principal {
            principal_id: credential.principal_id.clone(),
            ip,
        },
        credential_id: credential.credential_id.clone(),
        principal_id: credential.principal_id.clone(),
        status: PrincipalStatus::Active,
    };
    let request = AppRequest::UpdatePrincipalCredentialStatus(status_update);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::UpdatePrincipalCredentialStatus(_) => {
                ApiResponse::new(ApiStatus::Success(SuccessStatus::OperationCompleted), None)
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to update credential status");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to update credential".into(),
                )))
            }
            other_api_response => {
                tracing::error!(?other_api_response, "Unexpected AppResponse variant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Unexpected))
            }
        },
        Err(e) => {
            tracing::error!(?e);
            match e {
                Action(action_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        action_error.to_string().into(),
                    )));
                }
                Raft(raft_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        raft_error.to_string().into(),
                    )));
                }
            }
        }
    }
}

pub async fn grant_app_access_user<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: Json<AppGrantData>,
    auth: AuthAdminMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let principal = match db
        .principal()
        .get_by_principal_id(&req.0.principal_id, PrincipalStatus::Active)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::error!(principal_id = req.0.principal_id, "No principal found");
            return ApiResponse::error(ErrorStatus::OperationFailed(
                ErrorReason::PrincipalNotFound,
            ));
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let timestamp = now_ts();
    let mut principal_app_grants = CreatePrincipalAppGrants(vec![]);
    for app_id in req.0.apps {
        let app_grant = CreatePrincipalAppGrant {
            actor: auth.actor(&conn),
            principal_id: principal.principal_id.clone(),
            app_id: app_id.clone(),
            granted_at: timestamp,
            granted_by: Some(auth.user.id),
            revoked_at: None,
            revoked_by: None,
        };
        principal_app_grants.0.push(app_grant);
    }
    let request = AppRequest::CreatePrincipalAppGrants(principal_app_grants);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreatePrincipalAppGrants(_) => ApiCreateAppGrantResponse::new(),
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to create app grants");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to create app grant".into(),
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
                Action(action_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        action_error.to_string().into(),
                    )));
                }
                Raft(raft_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        raft_error.to_string().into(),
                    )));
                }
            }
        }
    }
}

pub async fn revoke_app_access_user<D: Database + 'static>(
    conn: ConnectionInfo,
    app: Data<AppData<D>>,
    req: Json<RevokeAppData>,
    auth: AuthAdminMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let principal_app_grant = match db
        .principal_app_grant()
        .get_by_app_id_and_principal_id(&req.0.app_id, &req.0.principal_id)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::error!(
                principal_id = req.0.principal_id,
                "No principal app grant found"
            );
            return ApiResponse::error(ErrorStatus::OperationFailed(
                ErrorReason::PrincipalNotFound,
            ));
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    let delete_app_grant = DeletePrincipalAppGrant {
        actor: auth.actor(&conn),
        principal_id: principal_app_grant.principal_id,
        app_id: principal_app_grant.app_id,
    };
    let request = AppRequest::DeletePrincipalAppGrant(delete_app_grant);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::DeletePrincipalAppGrant(_) => ApiDeleteAppGrantResponse::new(),
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to revoke app grant");
                ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                    "Failed to revoke app grant".into(),
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
                Action(action_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        action_error.to_string().into(),
                    )));
                }
                Raft(raft_error) => {
                    return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Message(
                        raft_error.to_string().into(),
                    )));
                }
            }
        }
    }
}

pub async fn list_app_access_user<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<ListAppsData>,
    _auth: AuthAdminMiddleware<D>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
    };

    let principal_app_grants = match db
        .principal_app_grant()
        .get_by_principal_id_after(&req.0.principal_id, req.0.after_app_id.as_deref(), 64)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::OperationFailed(ErrorReason::Database));
        }
    };

    ApiListAppsResponse::new(principal_app_grants)
}
