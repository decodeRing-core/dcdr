use std::fmt;

use serde::{Deserialize, Serialize};

use crate::actions::create_api_key::CreateApiKey;
use crate::actions::create_app::CreateApp;
use crate::actions::create_secret_mapping::CreateSecretMapping;
use crate::actions::create_shamir_configuration::CreateShamirConfiguration;
use crate::actions::create_user::CreateUser;
use crate::actions::system_init::SystemInit;
use crate::error::ActionError;
use crate::response::AppResponse;
use crate::runner::run_action_direct;
use crate::tx::Database;

#[derive(Debug, Serialize, Deserialize)]
pub enum AppRequest {
    CreateApiKey(CreateApiKey),
    CreateApp(CreateApp),
    CreateUser(CreateUser),
    CreateSecretMapping(CreateSecretMapping),
    CreateShamirConfiguration(CreateShamirConfiguration),
    SystemInit(SystemInit),
}

impl fmt::Display for AppRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppRequest::CreateApp(create_app) => {
                write!(
                    f,
                    "CreateApp(app_id={}, app_name={})",
                    create_app.app_id, create_app.app_name
                )
            }
            AppRequest::CreateUser(create_user) => {
                write!(
                    f,
                    "CreateUser(username={}, email={}, is_admin={})",
                    create_user.username, create_user.email, create_user.is_admin
                )
            }
            AppRequest::CreateApiKey(create_api_key) => {
                write!(f, "CreateApiKey(user_id={})", create_api_key.user_id)
            }
            AppRequest::CreateSecretMapping(create_secret_mapping) => {
                write!(
                    f,
                    "CreateSecretMapping(app_id={}, secret_name={}, backend={}, mount_path={}, tainted={})",
                    create_secret_mapping.app_id,
                    create_secret_mapping.secret_name,
                    create_secret_mapping.backend,
                    create_secret_mapping.mount_path,
                    create_secret_mapping.tainted
                )
            }
            AppRequest::CreateShamirConfiguration(create_shamir_configuration) => {
                write!(
                    f,
                    "CreateShamirConfiguration(total_shares={}, threshold={})",
                    create_shamir_configuration.total_shares, create_shamir_configuration.threshold
                )
            }
            AppRequest::SystemInit(_) => {
                write!(f, "SystemInit()")
            }
        }
    }
}

impl AppRequest {
    pub async fn run_direct<D>(self, db: &D) -> Result<AppResponse, ActionError>
    where
        D: Database,
    {
        match self {
            AppRequest::CreateApiKey(create_api_key) => {
                Ok(run_action_direct(db, create_api_key).await?.response)
            }
            AppRequest::CreateUser(create_user) => {
                Ok(run_action_direct(db, create_user).await?.response)
            }
            AppRequest::CreateApp(create_app) => {
                Ok(run_action_direct(db, create_app).await?.response)
            }
            AppRequest::CreateShamirConfiguration(create_shamir_configuration) => {
                Ok(run_action_direct(db, create_shamir_configuration)
                    .await?
                    .response)
            }
            AppRequest::CreateSecretMapping(create_secret_mapping) => {
                Ok(run_action_direct(db, create_secret_mapping).await?.response)
            }
            AppRequest::SystemInit(system_init) => {
                Ok(run_action_direct(db, system_init).await?.response)
            }
        }
    }
}
