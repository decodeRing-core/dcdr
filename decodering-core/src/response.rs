use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::{PrincipalCredentialKind, PrincipalKind, PrincipalStatus};

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateAppResponse {
    pub app_id: String,
    pub app_name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateUserResponse {
    pub username: String,
    pub email: String,
    pub is_admin: bool,
    pub created_at: i64,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateSecretMappingResponse {
    pub app_id: String,
    pub secret_name: String,
    pub backend: String,
    pub mount_path: String,
    pub tainted: i16,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateShamirConfigurationResponse {
    pub total_shares: i16,
    pub threshold: i16,
    pub timestamp: i64,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePluginConfigResponse {
    pub backend_name: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateApiKeyResponse {
    pub user_id: i64,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct SystemInitResponse {
    pub shamir: CreateShamirConfigurationResponse,
    pub user: CreateUserResponse,
    pub api_key: CreateApiKeyResponse,
    pub plugin_config: Vec<CreatePluginConfigResponse>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalResponse {
    pub principal_id: String,
    pub name: String,
    pub kind: PrincipalKind,
    pub status: PrincipalStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalCredentialResponse {
    pub credential_id: String,
    pub principal_id: String,
    pub kind: PrincipalCredentialKind,
    pub lookup_key: String,
    pub status: PrincipalStatus,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalAppGrantResponse {
    pub principal_id: String,
    pub app_id: String,
    pub granted_at: i64,
    pub granted_by: Option<i64>,
    pub revoked_at: Option<i64>,
    pub revoked_by: Option<i64>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalTokenResponse {
    pub token_id: String,
    pub principal_id: String,
    pub credential_id: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateTpmChallengeResponse {
    pub challenge_id: String,
    pub nonce: Vec<u8>,
    pub ek_pubkey_hash: Option<String>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct ConsumeTpmChallengeResponse {
    pub challenge_id: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateAppUserResponse {
    pub principal: CreatePrincipalResponse,
    pub principal_credential: CreatePrincipalCredentialResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AppResponse {
    CreateApp(CreateAppResponse),
    CreateUser(CreateUserResponse),
    CreateApiKey(CreateApiKeyResponse),
    CreateSecretMapping(CreateSecretMappingResponse),
    DeleteSecretMapping(bool),
    UpdateSecretMappingTaint(bool),
    CreateShamirConfiguration(CreateShamirConfigurationResponse),
    CreatePrincipal(CreatePrincipalResponse),
    CreatePrincipalCredential(CreatePrincipalCredentialResponse),
    UpdatePrincipalCredentialLastUsed(i64),
    CreatePrincipalToken(CreatePrincipalTokenResponse),
    CreatePrincipalAppGrant(CreatePrincipalAppGrantResponse),
    CreatePrincipalAppGrants(Vec<CreatePrincipalAppGrantResponse>),
    DeletePrincipalAppGrant(bool),
    CreatePluginConfig(CreatePluginConfigResponse),
    SystemInit(SystemInitResponse),
    CreateAppUser(CreateAppUserResponse),
    CreateTpmChallenge(CreateTpmChallengeResponse),
    ConsumeTpmChallenge(ConsumeTpmChallengeResponse),
    Noop,
    Error(String),
}

impl fmt::Display for AppResponse {
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
            Self::CreatePrincipal(create_principal) => {
                write!(
                    f,
                    "CreatePrincipal(principal_id={})",
                    create_principal.principal_id
                )
            }
            Self::CreatePrincipalToken(create_principal_token) => {
                write!(
                    f,
                    "CreatePrincipalToken(principal_id={})",
                    create_principal_token.principal_id
                )
            }
            Self::CreatePrincipalCredential(create_principal_credential_response) => {
                write!(
                    f,
                    "CreatePrincipalCredential(credential_id={}, principal_id={})",
                    create_principal_credential_response.credential_id,
                    create_principal_credential_response.principal_id
                )
            }
            Self::CreateTpmChallenge(create_tpm_challenge) => {
                write!(
                    f,
                    "CreateTpmChallenge(challenge_id={})",
                    create_tpm_challenge.challenge_id
                )
            }
            Self::ConsumeTpmChallenge(consume_tpm_challenge) => {
                write!(
                    f,
                    "ConsumeTpmChallenge(challenge_id={})",
                    consume_tpm_challenge.challenge_id
                )
            }
            Self::CreatePrincipalAppGrant(principal_app_grant) => {
                write!(
                    f,
                    "CreatePrincipalAppGrant(principal_id={}, app_id={})",
                    principal_app_grant.principal_id, principal_app_grant.app_id
                )
            }
            Self::CreatePrincipalAppGrants(principal_app_grants) => {
                write!(
                    f,
                    "CreatePrincipalAppGrants(total={})",
                    principal_app_grants.len()
                )
            }
            Self::CreatePluginConfig(plugin_config) => {
                write!(
                    f,
                    "CreatePluginConfig(backend_name={})",
                    plugin_config.backend_name
                )
            }
            Self::DeletePrincipalAppGrant(r) => write!(f, "DeletePrincipalAppGrant(deleted={r})"),
            Self::DeleteSecretMapping(r) => write!(f, "DeleteSecretMapping(deleted={r})"),
            Self::UpdatePrincipalCredentialLastUsed(r) => {
                write!(f, "UpdatePrincipalCredentialLastUsed(last_used_at={r})")
            }
            Self::CreateAppUser(_) => write!(f, "CreateAppUser()"),
            Self::Noop => write!(f, "Noop"),
            Self::Error(e) => write!(f, "Error({e})"),
            Self::SystemInit(_) => write!(f, "SystemInit()"),
            Self::UpdateSecretMappingTaint(r) => write!(f, "UpdateSecretMappingTaint(updated={r})"),
        }
    }
}
