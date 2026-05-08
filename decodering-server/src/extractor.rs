use std::marker::PhantomData;
use std::pin::Pin;

use actix_web::Error;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::web::Data;
use decodering_core::repository::Principal;
use decodering_core::repository::PrincipalRepository;
use decodering_core::repository::User;
use decodering_core::repository::UserRepository;
use decodering_core::sha256_hex;
use decodering_core::tx::Database;
use decodering_core::tx::Tx;
use serde::Serialize;

use crate::app_data::AppData;
use crate::handlers::response::ErrorStatus;

#[derive(Debug, Serialize)]
pub(crate) struct AuthAdminMiddleware<D> {
    pub(crate) user: User,
    _marker: PhantomData<D>,
}

impl<D> AuthAdminMiddleware<D> {
    pub fn new(user: User) -> Self {
        Self {
            user,
            _marker: PhantomData,
        }
    }
}

impl<D> FromRequest for AuthAdminMiddleware<D>
where
    D: Database + 'static,
{
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_c = req.clone();
        Box::pin(async move {
            let app = req_c
                .app_data::<Data<AppData<D>>>()
                .ok_or_else(|| actix_web::error::ErrorInternalServerError("AppData missing"))?
                .clone();

            let access_token = get_authorization(&req_c)?;
            let db = app.db.begin().await;
            let Ok(mut db) = db else {
                tracing::error!("Failed to get a connection to database");
                return Err(ErrorStatus::Internal.into());
            };

            let api_key_hash = sha256_hex(access_token.as_bytes());
            let user = db.user().get_admin_by_api_key(&api_key_hash).await;
            match user {
                Ok(Some(u)) => Ok(AuthAdminMiddleware::new(u)),
                Ok(None) => {
                    tracing::warn!(
                        access_token,
                        "Invalid authentication attempt. No user found for token"
                    );
                    Err(ErrorStatus::Unauthorized.into())
                }
                Err(e) => {
                    tracing::error!(err=%e, "Failed to query database");
                    Err(ErrorStatus::Internal.into())
                }
            }
        })
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthOSLMiddleware<D> {
    pub(crate) user: Option<User>,
    pub(crate) principal: Option<Principal>,
    _marker: PhantomData<D>,
}

impl<D> AuthOSLMiddleware<D> {
    pub fn new(user: Option<User>, principal: Option<Principal>) -> Self {
        Self {
            user,
            principal,
            _marker: PhantomData,
        }
    }
}

impl<D> FromRequest for AuthOSLMiddleware<D>
where
    D: Database + 'static,
{
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req_c = req.clone();
        Box::pin(async move {
            let app = req_c
                .app_data::<Data<AppData<D>>>()
                .ok_or_else(|| actix_web::error::ErrorInternalServerError("AppData missing"))?
                .clone();

            let access_token = get_authorization(&req_c)?;
            let db = app.db.begin().await;
            let Ok(mut db) = db else {
                tracing::error!("Failed to get a connection to database");
                return Err(ErrorStatus::Internal.into());
            };

            let api_key_hash = sha256_hex(access_token.as_bytes());

            if let Some(u) = db.user().get_by_api_key(&api_key_hash).await.map_err(|e| {
                tracing::error!(err=%e, "Failed to query database");
                ErrorStatus::Internal
            })? {
                return Ok(AuthOSLMiddleware::new(Some(u), None));
            }

            if let Some(p) = db
                .principal()
                .get_active_by_token(&api_key_hash)
                .await
                .map_err(|e| {
                    tracing::error!(err=%e, "Failed to query database");
                    ErrorStatus::Internal
                })?
            {
                return Ok(AuthOSLMiddleware::new(None, Some(p)));
            }

            tracing::warn!(
                access_token,
                "Invalid authentication attempt. No user or principal found for token"
            );
            Err(ErrorStatus::Unauthorized.into())
        })
    }
}

pub(crate) fn get_authorization(req: &HttpRequest) -> Result<String, Error> {
    let authorization = req.headers().get(header::AUTHORIZATION);
    let Some(authorization) = authorization else {
        return Err(ErrorStatus::Unauthorized.into());
    };

    let access_token = authorization.to_str();
    let Ok(mut access_token) = access_token else {
        return Err(ErrorStatus::Unauthorized.into());
    };

    let bearer_token: Vec<&str> = access_token.split_whitespace().collect();
    if bearer_token.len() != 2 {
        return Err(ErrorStatus::Unauthorized.into());
    }
    if bearer_token[0] != "Bearer" {
        return Err(ErrorStatus::Unauthorized.into());
    }
    access_token = bearer_token[1].trim();
    Ok(access_token.to_owned())
}
