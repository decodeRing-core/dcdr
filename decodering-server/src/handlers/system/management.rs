use crate::app_data::AppData;
use crate::handlers::app::payload::UnlockData;
use crate::handlers::response::{ApiResponse, ApiStatus, ErrorStatus, SuccessStatus};
use crate::handlers::system::payloads::InitSystemRequestData;
use crate::handlers::system::response::ApiInitSystemResponse;
use crate::shamir::{initialize_shamir, unlock};
use actix_web::Responder;
use actix_web::web::{Data, Json};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use blahaj::Share;
use decodering_core::actions::create_api_key::CreateApiKey;
use decodering_core::actions::create_shamir_configuration::CreateShamirConfiguration;
use decodering_core::actions::create_user::CreateUser;
use decodering_core::actions::system_init::SystemInit;
use decodering_core::repository::ShamirRepository;
use decodering_core::response::AppResponse;
use decodering_core::tx::{Database, Tx};
use rand::distr::{Alphanumeric, SampleString};
use zeroize::Zeroizing;

pub(crate) async fn system_init<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<InitSystemRequestData>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to database");
        return ApiResponse::error(ErrorStatus::Internal.into());
    };
    let shamir_configuration = db.shamir().get_first().await;
    let total_shares = match shamir_configuration {
        Ok(Some(_)) => 1,
        Ok(None) => 0,
        Err(e) => {
            tracing::error!(error=%e, "Database error");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    };

    let shamir_setup = if total_shares == 0 {
        tracing::debug!("Initializing shamir secret sharing");

        let Some(total_shares) = req.total_shares else {
            tracing::error!("Shard data initialization required. Missing total shares.");
            return ApiResponse::error(ErrorStatus::Internal.into());
        };
        let Some(threshold) = req.threshold else {
            tracing::error!("Shard data initialization required. Missing threshold.");
            return ApiResponse::error(ErrorStatus::Internal.into());
        };

        let shamir_init = match initialize_shamir(total_shares, threshold) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error=%e, "Shamir secret sharing initialization failed");
                return ApiResponse::error(ErrorStatus::Internal.into());
            }
        };
        Some((shamir_init, total_shares, threshold))
    } else {
        None
    };

    let Some((shamir_init, total_shares, threshold)) = shamir_setup else {
        tracing::error!("System already initialized");
        return ApiResponse::error(ErrorStatus::AlreadyInitialized.into());
    };

    let token = Alphanumeric.sample_string(&mut rand::rng(), 26);
    let shamir =
        CreateShamirConfiguration::new(total_shares as i16, threshold as i16, shamir_init.hash);
    let user = CreateUser::new("root", "root@localhost", "", 1);
    let api_key = CreateApiKey::init(token, None);
    let request_initialize = SystemInit::request(shamir, user, api_key);
    match app.submit(request_initialize).await {
        Ok(resp) => match resp {
            AppResponse::SystemInit(resp) => {
                let shards: Vec<String> = shamir_init
                    .shards
                    .iter()
                    .map(|sh| STANDARD.encode(Vec::from(sh)))
                    .collect();
                return ApiInitSystemResponse::initialized(
                    shards,
                    Some(resp.api_key.api_key.clone()),
                );
            }
            AppResponse::Error(e) => {
                tracing::error!(%e, "Failed to initialize system");
                return ApiResponse::error(ErrorStatus::Internal.into());
            }
            other_api_response => {
                tracing::error!(?other_api_response, "unexpected AppResponse variant");
                return ApiResponse::error(ErrorStatus::Internal.into());
            }
        },
        Err(e) => {
            tracing::error!(%e, "Failed to initialize system");
            return ApiResponse::error(ErrorStatus::Internal.into());
        }
    }
}

pub(crate) async fn system_unlock<D: Database + 'static>(
    app: Data<AppData<D>>,
    req: Json<UnlockData>,
) -> impl Responder {
    let db = app.db.begin().await;
    let Ok(mut db) = db else {
        tracing::error!("Failed to get a connection to DB");
        return ApiResponse::<()>::error(ErrorStatus::Internal.into());
    };
    let shamir_configuration = db.shamir().get_first().await;
    let Ok(Some(shamir_configuration)) = shamir_configuration else {
        if shamir_configuration.is_err() {
            tracing::error!(error=%shamir_configuration.unwrap_err(),"Failed to unlock node");
        }
        return ApiResponse::error(ErrorStatus::Internal.into());
    };
    let shares = req
        .0
        .shards
        .iter()
        .map(|bytes| Share::try_from(bytes.as_slice()))
        .collect::<Result<Vec<_>, _>>();

    let Ok(shares) = shares else {
        tracing::error!("Failed to unlock node. Failed to process shards.");
        return ApiResponse::error(ErrorStatus::Internal.into());
    };
    let threshold = u8::try_from(shamir_configuration.threshold);
    let Ok(threshold) = threshold else {
        tracing::error!("Shamir configuration threshold out of range");
        return ApiResponse::error(ErrorStatus::InvalidKeys.into());
    };
    let out = unlock(threshold, &shamir_configuration.validation_hash, shares);
    if let Ok(master_key) = out {
        let out = app.master_key.set(Zeroizing::new(master_key));
        match out {
            Ok(_) => return ApiResponse::empty(SuccessStatus::SystemUnlocked.into()),
            Err(_) => return ApiResponse::error(ErrorStatus::InvalidKeys.into()),
        }
    };
    tracing::error!(error=%out.unwrap_err(), "Failed to unlock node");
    ApiResponse::error(ErrorStatus::Internal.into())
}

pub(crate) async fn system_status<D: Database + 'static>(app: Data<AppData<D>>) -> impl Responder {
    if app.master_key.get().is_none() {
        return ApiResponse::<()>::empty(ErrorStatus::Locked.into());
    }
    return ApiResponse::<()>::empty(SuccessStatus::SystemUnlocked.into());
}
