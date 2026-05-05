use std::marker::PhantomData;
use std::pin::Pin;

use actix_web::Error;
use actix_web::FromRequest;
use actix_web::HttpRequest;
use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::web::Data;
use decodering_core::repository::User;
use decodering_core::repository::UserRepository;
use decodering_core::tx::Database;
use decodering_core::tx::Tx;
use serde::Serialize;

use crate::app_data::AppData;
use crate::handlers::response::ErrorStatus;

#[derive(Debug, Serialize)]
pub(crate) struct AuthMiddleware<D> {
    pub(crate) user: User,
    _marker: PhantomData<D>,
}

impl<D> AuthMiddleware<D> {
    pub fn new(user: User) -> Self {
        Self {
            user,
            _marker: PhantomData,
        }
    }
}

impl<D> FromRequest for AuthMiddleware<D>
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

            let user = db.user().get_by_api_key(&access_token).await;
            match user {
                Ok(Some(u)) => Ok(AuthMiddleware::new(u)),
                Ok(None) => {
                    tracing::warn!(
                        access_token,
                        "Invalid authentication attempt. No user found for token"
                    );
                    return Err(ErrorStatus::Unauthorized.into());
                }
                Err(e) => {
                    tracing::error!(err=%e, "Failed to query database");
                    return Err(ErrorStatus::Internal.into());
                }
            }
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
