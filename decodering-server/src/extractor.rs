use std::marker::PhantomData;
use std::pin::Pin;

use actix_web::Error;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::dev::ConnectionInfo;
use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::web::Data;
use decodering_core::audit::Actor;
use decodering_core::crypto::sha256_hex;
use decodering_core::repository::Principal;
use decodering_core::repository::PrincipalRepository;
use decodering_core::repository::User;
use decodering_core::repository::UserRepository;
use decodering_core::tx::Database;
use decodering_core::tx::Tx;
use serde::Serialize;

use crate::app_data::AppData;
use crate::error::ErrorReason;
use crate::handlers::response::ErrorStatus;

#[derive(Debug, Serialize)]
pub struct AuthAdminMiddleware<D> {
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

    pub fn actor(&self, conn: &ConnectionInfo) -> Actor {
        let ip = conn.peer_addr().map(str::to_owned);
        Actor::User {
            user_id: self.user.id,
            ip,
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
                return Err(ErrorStatus::OperationFailed(ErrorReason::Database).into());
            };

            let api_key_hash = sha256_hex(access_token.as_bytes());
            let user = db.user().get_admin_by_api_key(&api_key_hash).await;
            match user {
                Ok(Some(u)) => Ok(Self::new(u)),
                Ok(None) => {
                    tracing::warn!(
                        access_token,
                        "Invalid authentication attempt. No user found for token"
                    );
                    Err(ErrorStatus::OperationFailed(ErrorReason::Unauthorized).into())
                }
                Err(e) => {
                    tracing::error!(err=%e, "Failed to query database");
                    Err(ErrorStatus::OperationFailed(ErrorReason::Database).into())
                }
            }
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AuthOSLMiddleware<D> {
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

    pub fn actor(&self, conn: &ConnectionInfo) -> Actor {
        let ip = conn.peer_addr().map(str::to_owned);
        if let Some(ref user) = self.user {
            return Actor::User {
                user_id: user.id,
                ip,
            };
        }
        if let Some(ref principal) = self.principal {
            return Actor::Principal {
                principal_id: principal.principal_id.clone(),
                ip,
            };
        }
        Actor::None { ip }
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
                return Err(ErrorStatus::OperationFailed(ErrorReason::Database).into());
            };

            let api_key_hash = sha256_hex(access_token.as_bytes());

            // User exists and is admin
            if let Some(u) = db.user().get_by_api_key(&api_key_hash).await.map_err(|e| {
                tracing::error!(err=%e, "Failed to query database");
                ErrorStatus::OperationFailed(ErrorReason::Database)
            })? && u.is_admin
            {
                return Ok(Self::new(Some(u), None));
            }

            // OR token is from a principal and has been granted access to app id (checked on handler)
            // Here we only check if the token is valid
            if let Some(p) = db
                .principal()
                .get_active_by_token(&api_key_hash)
                .await
                .map_err(|e| {
                    tracing::error!(err=%e, "Failed to query database");
                    ErrorStatus::OperationFailed(ErrorReason::Database)
                })?
            {
                return Ok(Self::new(None, Some(p)));
            }

            tracing::warn!(
                access_token,
                "Invalid authentication attempt. No user or principal found for token"
            );
            Err(ErrorStatus::OperationFailed(ErrorReason::Unauthorized).into())
        })
    }
}

pub fn get_authorization(req: &HttpRequest) -> Result<String, Error> {
    let authorization = req.headers().get(header::AUTHORIZATION);
    let Some(authorization) = authorization else {
        return Err(ErrorStatus::OperationFailed(ErrorReason::Unauthorized).into());
    };

    let access_token = authorization.to_str();
    let Ok(access_token) = access_token else {
        return Err(ErrorStatus::OperationFailed(ErrorReason::Unauthorized).into());
    };

    if access_token.split_whitespace().count() != 2 {
        return Err(ErrorStatus::OperationFailed(ErrorReason::Unauthorized).into());
    }
    let token = access_token
        .strip_prefix("Bearer ")
        .ok_or(ErrorStatus::OperationFailed(ErrorReason::Unauthorized))?;

    if token.is_empty() || token.contains(char::is_whitespace) {
        return Err(ErrorStatus::OperationFailed(ErrorReason::Unauthorized).into());
    }

    Ok(token.to_owned())
}
