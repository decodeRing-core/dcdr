use actix_web::Responder;
use actix_web::web::{Data, Json};
use decodering_core::actions::create_app::CreateApp;
use decodering_core::actions::create_app_user::CreateAppUser;
use decodering_core::actions::create_principal::CreatePrincipal;
use decodering_core::actions::create_principal_app_grant::{
    CreatePrincipalAppGrant, CreatePrincipalAppGrants,
};
use decodering_core::actions::create_principal_credential::CreatePrincipalCredential;
use decodering_core::actions::create_principal_token::CreatePrincipalToken;
use decodering_core::actions::create_tpm_challenge::CreateTpmChallenge;
use decodering_core::cert::{TpmTrustStore, verify_ek_cert_chain};
use decodering_core::crypto::{encode_hex, pem_to_der, sha256_hex};
use decodering_core::domain::{PrincipalCredentialKind, PrincipalStatus};
use decodering_core::repository::{AppRepository, PrincipalRepository};
use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::time::{CHALLENGE_TTL_SECS, now_ts, now_ts_plus};
use decodering_core::tx::{Database, Tx};
use rand::Rng;
use rand::distr::{Alphanumeric, SampleString};
use uuid::Uuid;

use crate::app_data::AppData;
use crate::config::get_config;
use crate::extractor::AuthAdminMiddleware;
use crate::handlers::app::payload::{
    AppGrantData, AuthTpmData, AuthUserData, CreateAppData, CreateAppUserData,
};
use crate::handlers::app::response::{
    ApiAuthAppUserResponse, ApiCreateAppGrantResponse, ApiTpmChallengeResponse,
};
use crate::handlers::app::response::{ApiCreateAppResponse, ApiCreateAppUserResponse};
use crate::handlers::response::{ApiResponse, ErrorStatus};

pub async fn create_app_user<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<CreateAppUserData>,
    auth: AuthAdminMiddleware<D>,
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

    let timestamp = now_ts();
    let principal_id = Uuid::now_v7().to_string();
    let principal = CreatePrincipal {
        principal_id: principal_id.clone(),
        name: req.0.name,
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
        PrincipalCredentialKind::TrustedPlatformModule => {
            let Some(ref tpm_req) = req.0.tpm else {
                tracing::error!("Missing TPM data");
                return ApiResponse::error(ErrorStatus::Internal);
            };
            let ek_der = match pem_to_der(&tpm_req.ek_pubkey_pem) {
                Ok(x) => x,
                Err(e) => {
                    tracing::error!(err=?e, "Invalid EK public key");
                    return ApiResponse::error(ErrorStatus::Internal);
                }
            };
            let ek_hash = sha256_hex(&ek_der);
            if tpm_req.require_ek_cert {
                let cert_pem = tpm_req.ek_cert_pem.as_ref();
                let Some(cert_pem) = cert_pem else {
                    tracing::error!("EK cert required");
                    return ApiResponse::error(ErrorStatus::Internal);
                };
                let config = get_config();
                let trust_store = TpmTrustStore::from_directory(&config.tpm_trust_dir);
                let Ok(trust_store) = trust_store else {
                    tracing::error!("Failed to load trust store");
                    return ApiResponse::error(ErrorStatus::Internal);
                };
                tracing::info!(count = trust_store.len(), "loaded TPM trust anchors");

                if let Err(e) = verify_ek_cert_chain(cert_pem, &tpm_req.ek_pubkey_pem, &trust_store)
                {
                    tracing::error!(err=?e, "EK cert verification failed");
                    return ApiResponse::error(ErrorStatus::Internal);
                }
            }

            ("TPM key added".to_owned(), ek_hash)
        }
        PrincipalCredentialKind::AwsIdentity => {
            return ApiResponse::error(ErrorStatus::Unimplemented);
        }
    };

    let secret_material = match req.0.credential_kind {
        PrincipalCredentialKind::ApiKey => serde_json::json!({}),
        PrincipalCredentialKind::TrustedPlatformModule => {
            let Some(ref tpm_req) = req.0.tpm else {
                tracing::error!("Missing TPM data");
                return ApiResponse::error(ErrorStatus::Internal);
            };
            let material = serde_json::json!({
                "ek_pubkey_pem":   tpm_req.ek_pubkey_pem,
                "ek_cert_pem":     tpm_req.ek_cert_pem,
                "require_ek_cert": tpm_req.require_ek_cert,
                "expected_pcrs":   tpm_req.expected_pcrs,
            });
            material
        }
        PrincipalCredentialKind::AwsIdentity => {
            return ApiResponse::error(ErrorStatus::Unimplemented);
        }
    };

    let principal_credential = CreatePrincipalCredential {
        credential_id: Uuid::now_v7().to_string(),
        principal_id: principal_id.clone(),
        kind: req.0.credential_kind,
        lookup_key,
        secret_material: secret_material.to_string(),
        status: PrincipalStatus::Active,
        expires_at: req.0.expires_at,
        last_used_at: None,
        created_at: timestamp,
        revoked_at: None,
    };

    let mut principal_app_grants = vec![];
    for app_id in req.0.apps.unwrap_or_default() {
        let app_grant = CreatePrincipalAppGrant {
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
        auth.user.id,
        principal,
        principal_credential,
        principal_app_grants,
    );
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreateAppUser(_) => ApiCreateAppUserResponse::new(token, principal_id),
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to create app user");
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

pub async fn create_app<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<CreateAppData>,
    _auth: AuthAdminMiddleware<D>,
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

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal);
    };
    match db.app().get_by_app_name(&req.0.app_name).await {
        Ok(Some(a)) => {
            tracing::warn!(
                name = a.app_name,
                id = a.app_id,
                "Cannot create a duplicate app with name"
            );
            return ApiResponse::error(ErrorStatus::DuplicatedApp);
        }
        Ok(None) => (),
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::Internal);
        }
    }

    let request = CreateApp::request(Uuid::now_v7().to_string(), req.0.app_name);
    match app.submit(request).await {
        Ok(resp) => match resp {
            AppResponse::CreateApp(a) => ApiCreateAppResponse::new(a.app_id, a.app_name),
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

pub async fn auth_app_user<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<AuthUserData>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal);
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
            return ApiResponse::error(ErrorStatus::Internal);
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::Internal);
        }
    };

    let token = format!("tok_{}", Alphanumeric.sample_string(&mut rand::rng(), 32));
    let token_hash = sha256_hex(token.as_bytes());

    let timestamp = now_ts();
    let expires = now_ts_plus(3600);
    let principal_token = CreatePrincipalToken {
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
                ApiAuthAppUserResponse::new(token, r.expires_at)
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

pub async fn auth_tpm_app_user<D: Database + 'static>(app: Data<AppData<D>>) -> impl Responder {
    ApiResponse::<()>::error(ErrorStatus::Unimplemented)
}

pub async fn tpm_challenge_app_user<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<AuthTpmData>,
) -> impl Responder {
    let mut nonce_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce_hex = encode_hex(&nonce_bytes);

    let challenge_id = Uuid::now_v7().to_string();
    let now = now_ts();
    let expires_at = now + CHALLENGE_TTL_SECS;

    let tpm_challenge = CreateTpmChallenge {
        challenge_id,
        nonce: nonce_bytes.to_vec(),
        ek_pubkey_hash: req.0.ek_pubkey_hash,
        issued_at: now,
        expires_at,
        consumed_at: None,
    };
    let tpm_request = AppRequest::CreateTpmChallenge(tpm_challenge);
    match app.submit(tpm_request).await {
        Ok(resp) => match resp {
            AppResponse::CreateTpmChallenge(r) => {
                ApiTpmChallengeResponse::new(r.challenge_id, nonce_hex, expires_at)
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

pub async fn auth_aws_iam_app_user<D: Database + 'static>(app: Data<AppData<D>>) -> impl Responder {
    ApiResponse::<()>::error(ErrorStatus::Unimplemented)
}

pub async fn grant_app_access_user<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<AppGrantData>,
    auth: AuthAdminMiddleware<D>,
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

    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal);
    };

    let principal = match db
        .principal()
        .get_by_principal_id(&req.0.principal_id, PrincipalStatus::Active)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::error!(principal_id = req.0.principal_id, "No principal found");
            return ApiResponse::error(ErrorStatus::Internal);
        }
        Err(e) => {
            tracing::error!(err=?e, "Failed to query database");
            return ApiResponse::error(ErrorStatus::Internal);
        }
    };

    let timestamp = now_ts();
    let mut principal_app_grants = CreatePrincipalAppGrants(vec![]);
    for app_id in req.0.apps {
        let app_grant = CreatePrincipalAppGrant {
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

pub async fn revoke_app_access_user<D: Database + 'static>(
    app: Data<AppData<D>>,
) -> impl Responder {
    ApiResponse::<()>::error(ErrorStatus::Unimplemented)
}
