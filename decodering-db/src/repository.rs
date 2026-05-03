use crate::domain::{AuditOutcome, PrincipalKind, PrincipalStatus};
use crate::error::DbError;

#[derive(Debug, Clone)]
pub struct PrincipalEntry {
    pub name: String,
    pub app_id: String,
    pub kind: PrincipalKind,
    pub status: PrincipalStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

pub trait PrincipalRepository: Send {
    fn insert(
        &mut self,
        principal: &PrincipalEntry,
    ) -> impl Future<Output = Result<i64, DbError>> + Send;
}

pub trait MetaRepository: Send {
    fn get(&mut self, key: &str) -> impl Future<Output = Result<Option<String>, DbError>> + Send;
    fn set(&mut self, key: &str, value: &str) -> impl Future<Output = Result<(), DbError>> + Send;
}

pub struct AuditEntry {
    pub raft_index: i64,
    pub timestamp: i64,

    pub user_id: Option<i64>,
    pub principal_id: Option<String>,

    pub action_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,

    pub outcome: AuditOutcome,
    pub reason: Option<String>,

    pub before_state: Option<String>,
    pub after_state: Option<String>,
    pub metadata: Option<String>,

    pub revertible: bool,
    pub undoes: Option<i64>,
}

pub trait AuditRepository: Send {
    fn insert(&mut self, audit: &AuditEntry) -> impl Future<Output = Result<i64, DbError>> + Send;
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Shamir {
    pub id: i64,
    pub total_shares: i16,
    pub threshold: i16,
    pub validation_hash: Vec<u8>,
    pub created_at: i64,
}

pub struct ShamirEntry {
    pub total_shares: i16,
    pub threshold: i16,
    pub validation_hash: Vec<u8>,
    pub created_at: i64,
}

pub trait ShamirRepository: Send {
    fn get_first(&mut self) -> impl Future<Output = Result<Option<Shamir>, DbError>> + Send;
    fn insert(&mut self, params: &ShamirEntry)
    -> impl Future<Output = Result<i64, DbError>> + Send;
}

pub struct ApiKeysEntry {
    pub user_id: i64,
    pub api_key: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

pub trait ApiKeysRepository: Send {
    fn insert(
        &mut self,
        params: &ApiKeysEntry,
    ) -> impl Future<Output = Result<i64, DbError>> + Send;
}

pub struct UserEntry {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: i64,
}

pub trait UserRepository: Send {
    fn insert(&mut self, params: &UserEntry) -> impl Future<Output = Result<i64, DbError>> + Send;
}

pub struct AppEntry {
    pub app_id: String,
    pub app_name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub trait AppRepository: Send {
    fn insert(&mut self, params: &AppEntry)
    -> impl Future<Output = Result<String, DbError>> + Send;
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SecretMapping {
    pub id: i64,
    pub app_id: String,
    pub secret_name: String,
    pub backend: String,
    pub mount_path: String,
    pub tainted: i16,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct SecretMappingEntry {
    pub app_id: String,
    pub secret_name: String,
    pub backend: String,
    pub mount_path: String,
    pub tainted: i16,
    pub created_at: i64,
    pub updated_at: i64,
}

pub trait SecretMappingRespository: Send {
    fn insert(
        &mut self,
        params: &SecretMappingEntry,
    ) -> impl Future<Output = Result<i64, DbError>> + Send;
    fn get_by_app_id_secret_name(
        &mut self,
        app_id: &str,
        secret_name: &str,
    ) -> impl Future<Output = Result<Option<SecretMapping>, DbError>> + Send;
}
