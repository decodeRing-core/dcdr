use std::fmt;

use serde::{Deserialize, Serialize};

use crate::domain::{PrincipalKind, PrincipalStatus};

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateAppResponse {
    pub app_id: String,
    pub app_name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateUserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub is_admin: u8,
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
    pub id: i64,
    pub total_shares: i16,
    pub threshold: i16,
    pub validation_hash: Vec<u8>,
    pub timestamp: i64,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateApiKeyResponse {
    pub user_id: i64,
    pub api_key: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub enum AppResponse {
    CreateApp(CreateAppResponse),
    CreateUser(CreateUserResponse),
    CreateApiKey(CreateApiKeyResponse),
    CreateSecretMapping(CreateSecretMappingResponse),
    CreateShamirConfiguration(CreateShamirConfigurationResponse),
    CreatePrincipal(CreatePrincipalResponse),
    SystemInit(SystemInitResponse),
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

            AppResponse::Noop => write!(f, "Noop"),
            AppResponse::Error(e) => write!(f, "Error({e})"),
            AppResponse::SystemInit(_) => write!(f, "SystemInit()",),
        }
    }
}
