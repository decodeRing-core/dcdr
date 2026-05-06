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
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalResponse {
    pub principal_id: String,
    pub name: String,
    pub app_id: String,
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
pub struct CreatePrincipalTokenResponse {
    pub token_id: String,
    pub principal_id: String,
    pub credential_id: Option<String>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
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
    CreateShamirConfiguration(CreateShamirConfigurationResponse),
    CreatePrincipal(CreatePrincipalResponse),
    CreatePrincipalCredential(CreatePrincipalCredentialResponse),
    CreatePrincipalToken(CreatePrincipalTokenResponse),
    SystemInit(SystemInitResponse),
    CreateAppUser(CreateAppUserResponse),
    Noop,
    Error(String),
}

impl fmt::Display for AppResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppResponse::CreateApp(create_app) => {
                write!(
                    f,
                    "CreateApp(app_id={}, app_name={})",
                    create_app.app_id, create_app.app_name
                )
            }
            AppResponse::CreateUser(create_user) => {
                write!(
                    f,
                    "CreateUser(username={}, email={}, is_admin={})",
                    create_user.username, create_user.email, create_user.is_admin
                )
            }
            AppResponse::CreateApiKey(create_api_key) => {
                write!(f, "CreateApiKey(user_id={})", create_api_key.user_id)
            }
            AppResponse::CreateSecretMapping(create_secret_mapping) => {
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
            AppResponse::CreateShamirConfiguration(create_shamir_configuration) => {
                write!(
                    f,
                    "CreateShamirConfiguration(total_shares={}, threshold={})",
                    create_shamir_configuration.total_shares, create_shamir_configuration.threshold
                )
            }
            AppResponse::CreatePrincipal(create_principal) => {
                write!(
                    f,
                    "CreatePrincipal(principal_id={})",
                    create_principal.principal_id
                )
            }
            AppResponse::CreatePrincipalToken(create_principal_token) => {
                write!(
                    f,
                    "CreatePrincipalToken(principal_id={})",
                    create_principal_token.principal_id
                )
            }
            AppResponse::CreatePrincipalCredential(create_principal_credential_response) => {
                write!(
                    f,
                    "CreatePrincipalCredential(credential_id={}, principal_id={})",
                    create_principal_credential_response.credential_id,
                    create_principal_credential_response.principal_id
                )
            }
            AppResponse::CreateAppUser(_) => write!(f, "CreateAppUser()"),
            AppResponse::Noop => write!(f, "Noop"),
            AppResponse::Error(e) => write!(f, "Error({e})"),
            AppResponse::SystemInit(_) => write!(f, "SystemInit()"),
            AppResponse::DeleteSecretMapping(r) => write!(f, "DeleteSecretMapping(deleted={})", r),
        }
    }
}
