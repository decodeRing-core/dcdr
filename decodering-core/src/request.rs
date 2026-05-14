use std::fmt;

use serde::{Deserialize, Serialize};

use crate::actions::create_api_key::CreateApiKey;
use crate::actions::create_app::CreateApp;
use crate::actions::create_app_user::CreateAppUser;
use crate::actions::create_principal::CreatePrincipal;
use crate::actions::create_principal_app_grant::CreatePrincipalAppGrants;
use crate::actions::create_principal_credential::CreatePrincipalCredential;
use crate::actions::create_principal_token::CreatePrincipalToken;
use crate::actions::create_secret_mapping::CreateSecretMapping;
use crate::actions::create_shamir_configuration::CreateShamirConfiguration;
use crate::actions::create_tpm_challenge::CreateTpmChallenge;
use crate::actions::create_user::CreateUser;
use crate::actions::delete_principal_app_grant::DeletePrincipalAppGrant;
use crate::actions::delete_secret_mapping::DeleteSecretMapping;
use crate::actions::system_init::SystemInit;
use crate::actions::update_consumed_at::UpdateConsumedAt;
use crate::actions::update_principal_credential_last_used::UpdatePrincipalCredentialLastUsed;
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
    CreatePrincipalAppGrants(CreatePrincipalAppGrants),
    DeletePrincipalAppGrant(DeletePrincipalAppGrant),
    CreateAppUser(CreateAppUser),
    CreateTpmChallenge(CreateTpmChallenge),
    UpdateConsumedAt(UpdateConsumedAt),
    UpdatePrincipalCredentialLastUsed(UpdatePrincipalCredentialLastUsed),
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
            Self::CreatePrincipalAppGrants(create_principal_app_grants) => {
                write!(
                    f,
                    "CreatePrincipalAppGrants(total={})",
                    create_principal_app_grants.0.len(),
                )
            }
            Self::DeletePrincipalAppGrant(delete_principal_app_grant) => {
                write!(
                    f,
                    "DeletePrincipalAppGrant(app_id={}, principal_id={})",
                    delete_principal_app_grant.app_id, delete_principal_app_grant.principal_id
                )
            }
            Self::UpdatePrincipalCredentialLastUsed(update_principal_credential_last_used) => {
                write!(
                    f,
                    "UpdatePrincipalCredentialLastUsed(credential_id={}, principal_id={})",
                    update_principal_credential_last_used.credential_id,
                    update_principal_credential_last_used.principal_id
                )
            }
            Self::CreatePrincipalToken(_) => {
                write!(f, "CreatePrincipalToken()")
            }
            Self::UpdateConsumedAt(_) => {
                write!(f, "UpdateConsumedAt()")
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
            Self::CreateApiKey(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreateUser(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreateApp(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreateAppUser(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreateShamirConfiguration(action) => {
                Ok(run_action_direct(db, action).await?.response)
            }
            Self::CreateSecretMapping(action) => Ok(run_action_direct(db, action).await?.response),
            Self::SystemInit(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreatePrincipal(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreatePrincipalCredential(action) => {
                Ok(run_action_direct(db, action).await?.response)
            }
            Self::DeleteSecretMapping(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreatePrincipalToken(action) => Ok(run_action_direct(db, action).await?.response),
            Self::CreatePrincipalAppGrants(action) => {
                Ok(run_action_direct(db, action).await?.response)
            }
            Self::DeletePrincipalAppGrant(action) => {
                Ok(run_action_direct(db, action).await?.response)
            }
            Self::CreateTpmChallenge(action) => Ok(run_action_direct(db, action).await?.response),
            Self::UpdateConsumedAt(action) => Ok(run_action_direct(db, action).await?.response),
            Self::UpdatePrincipalCredentialLastUsed(action) => {
                Ok(run_action_direct(db, action).await?.response)
            }
        }
    }
}
