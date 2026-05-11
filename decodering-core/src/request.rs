use std::fmt;

use serde::{Deserialize, Serialize};

use crate::actions::create_api_key::CreateApiKey;
use crate::actions::create_app::CreateApp;
use crate::actions::create_app_user::CreateAppUser;
use crate::actions::create_principal::CreatePrincipal;
use crate::actions::create_principal_credential::CreatePrincipalCredential;
use crate::actions::create_principal_token::CreatePrincipalToken;
use crate::actions::create_secret_mapping::CreateSecretMapping;
use crate::actions::create_shamir_configuration::CreateShamirConfiguration;
use crate::actions::create_tpm_challenge::CreateTpmChallenge;
use crate::actions::create_user::CreateUser;
use crate::actions::delete_secret_mapping::DeleteSecretMapping;
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
    DeleteSecretMapping(DeleteSecretMapping),
    CreateShamirConfiguration(CreateShamirConfiguration),
    CreatePrincipal(CreatePrincipal),
    CreatePrincipalCredential(CreatePrincipalCredential),
    CreatePrincipalToken(CreatePrincipalToken),
    CreateAppUser(CreateAppUser),
    CreateTpmChallenge(CreateTpmChallenge),
    SystemInit(SystemInit),
}

impl fmt::Display for AppRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateApp(create_app) => {
                write!(
                    f,
                    "CreateApp(app_id={}, app_name={})",
                    create_app.app_id, create_app.app_name
                )
            }
            Self::CreateUser(create_user) => {
                write!(
                    f,
                    "CreateUser(username={}, email={}, is_admin={})",
                    create_user.username, create_user.email, create_user.is_admin
                )
            }
            Self::CreateApiKey(create_api_key) => {
                write!(f, "CreateApiKey(user_id={})", create_api_key.user_id)
            }
            Self::CreateSecretMapping(create_secret_mapping) => {
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
            Self::CreateShamirConfiguration(create_shamir_configuration) => {
                write!(
                    f,
                    "CreateShamirConfiguration(total_shares={}, threshold={})",
                    create_shamir_configuration.total_shares, create_shamir_configuration.threshold
                )
            }
            Self::SystemInit(_) => {
                write!(f, "SystemInit()")
            }
            Self::CreateAppUser(_) => {
                write!(f, "CreateAppUser()")
            }
            Self::CreatePrincipal(create_principal) => {
                write!(
                    f,
                    "CreatePrincipal(name={}, principal_id={})",
                    create_principal.name, create_principal.principal_id
                )
            }
            Self::CreatePrincipalCredential(create_principal_credential) => {
                write!(
                    f,
                    "CreatePrincipalCredential(credential_id={}, principal_id={})",
                    create_principal_credential.credential_id,
                    create_principal_credential.principal_id
                )
            }
            Self::DeleteSecretMapping(delete_secret_mapping) => {
                write!(
                    f,
                    "DeleteSecretMapping(app_id={}, secret_name={})",
                    delete_secret_mapping.app_id, delete_secret_mapping.secret_name
                )
            }
            Self::CreateTpmChallenge(create_tpm_challenge) => {
                write!(
                    f,
                    "CreateTpmChallenge(challenge_id={})",
                    create_tpm_challenge.challenge_id,
                )
            }
            Self::CreatePrincipalToken(_) => {
                write!(f, "CreatePrincipalToken()")
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
            Self::CreateApiKey(create_api_key) => {
                Ok(run_action_direct(db, create_api_key).await?.response)
            }
            Self::CreateUser(create_user) => Ok(run_action_direct(db, create_user).await?.response),
            Self::CreateApp(create_app) => Ok(run_action_direct(db, create_app).await?.response),
            Self::CreateAppUser(create_app_user) => {
                Ok(run_action_direct(db, create_app_user).await?.response)
            }
            Self::CreateShamirConfiguration(create_shamir_configuration) => {
                Ok(run_action_direct(db, create_shamir_configuration)
                    .await?
                    .response)
            }
            Self::CreateSecretMapping(create_secret_mapping) => {
                Ok(run_action_direct(db, create_secret_mapping).await?.response)
            }
            Self::SystemInit(system_init) => Ok(run_action_direct(db, system_init).await?.response),
            Self::CreatePrincipal(create_principal) => {
                Ok(run_action_direct(db, create_principal).await?.response)
            }
            Self::CreatePrincipalCredential(create_principal_credential) => {
                Ok(run_action_direct(db, create_principal_credential)
                    .await?
                    .response)
            }
            Self::DeleteSecretMapping(delete_secret_mapping) => {
                Ok(run_action_direct(db, delete_secret_mapping).await?.response)
            }
            Self::CreatePrincipalToken(create_principal_token) => {
                Ok(run_action_direct(db, create_principal_token)
                    .await?
                    .response)
            }
            Self::CreateTpmChallenge(create_tpm_challenge) => {
                Ok(run_action_direct(db, create_tpm_challenge).await?.response)
            }
        }
    }
}
