use serde::Serialize;

use crate::domain::{AuditOutcome, PrincipalCredentialKind, PrincipalKind, PrincipalStatus};
use crate::error::DbError;

#[derive(Serialize, Debug, Clone)]
pub struct PrincipalAppGrant {
    pub principal_id: String,
    pub app_id: String,
    pub granted_at: i64,
    pub granted_by: Option<i64>,
    pub revoked_at: Option<i64>,
    pub revoked_by: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PrincipalAppGrantEntry {
    pub principal_id: String,
    pub app_id: String,
    pub granted_at: i64,
    pub granted_by: Option<i64>,
    pub revoked_at: Option<i64>,
    pub revoked_by: Option<i64>,
}

pub trait PrincipalAppGrantRepository: Send {
    fn insert_many(
        &mut self,
        principal_app_grants: &[PrincipalAppGrantEntry],
    ) -> impl Future<Output = Result<(), DbError>> + Send;
    fn insert(
        &mut self,
        principal_app_grant: &PrincipalAppGrantEntry,
    ) -> impl Future<Output = Result<String, DbError>> + Send;
    fn delete(
        &mut self,
        app_id: &str,
        principal_id: &str,
    ) -> impl Future<Output = Result<u64, DbError>> + Send;
    fn get_by_app_id_and_principal_id(
        &mut self,
        app_id: &str,
        principal_id: &str,
    ) -> impl Future<Output = Result<Option<PrincipalAppGrant>, DbError>> + Send;
    fn get_by_principal_id_after(
        &mut self,
        principal_id: &str,
        after_app_id: Option<&str>,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<PrincipalAppGrant>, DbError>> + Send;
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthChallenge {
    pub challenge_id: String,
    pub method: String,
    pub payload: Vec<u8>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AuthChallengeEntry {
    pub challenge_id: String,
    pub method: String,
    pub payload: Vec<u8>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
}

pub trait AuthChallengeRepository: Send {
    fn insert(
        &mut self,
        auth_challenge: &AuthChallengeEntry,
    ) -> impl Future<Output = Result<String, DbError>> + Send;
    fn update_consumed(
        &mut self,
        challenge_id: &str,
        consumed_at: i64,
    ) -> impl Future<Output = Result<String, DbError>> + Send;
    fn get_active(
        &mut self,
        challenge_id: &str,
    ) -> impl Future<Output = Result<Option<AuthChallenge>, DbError>> + Send;
    fn delete_expired(&mut self) -> impl Future<Output = Result<u64, DbError>> + Send;
}

#[derive(Debug, Clone)]
pub struct PrincipalTokenEntry {
    pub token_id: String,
    pub token_hash: String,
    pub principal_id: String,
    pub credential_id: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}

pub trait PrincipalTokenRepository: Send {
    fn insert(
        &mut self,
        principal_credential: &PrincipalTokenEntry,
    ) -> impl Future<Output = Result<String, DbError>> + Send;
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalCredential {
    pub credential_id: String,
    pub principal_id: String,
    pub kind: PrincipalCredentialKind,
    pub lookup_key: String,
    pub secret_material: String,
    pub status: PrincipalStatus,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PrincipalCredentialEntry {
    pub credential_id: String,
    pub principal_id: String,
    pub kind: PrincipalCredentialKind,
    pub lookup_key: String,
    pub secret_material: String,
    pub status: PrincipalStatus,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

pub trait PrincipalCredentialRepository: Send {
    fn insert(
        &mut self,
        principal_credential: &PrincipalCredentialEntry,
    ) -> impl Future<Output = Result<String, DbError>> + Send;
    fn get_active_by_kind_and_lookup_key(
        &mut self,
        kind: PrincipalCredentialKind,
        lookup_key: &str,
    ) -> impl Future<Output = Result<Option<PrincipalCredential>, DbError>> + Send;
    fn get_pending_by_kind_and_credential_and_principal(
        &mut self,
        principal_id: String,
        credential_id: String,
        kind: PrincipalCredentialKind,
    ) -> impl Future<Output = Result<Option<PrincipalCredential>, DbError>> + Send;
    fn get_by_credential_and_principal(
        &mut self,
        credential_id: &str,
        principal_id: &str,
    ) -> impl Future<Output = Result<Option<PrincipalCredential>, DbError>> + Send;
    fn update_last_used(
        &mut self,
        credential_id: &str,
        last_used_at: i64,
    ) -> impl Future<Output = Result<u64, DbError>> + Send;
    fn update_status(
        &mut self,
        credential_id: &str,
        status: PrincipalStatus,
    ) -> impl Future<Output = Result<u64, DbError>> + Send;
}

#[derive(Serialize, Debug, Clone)]
pub struct Principal {
    pub credential_id: String,
    pub principal_id: String,
    pub name: String,
    pub kind: PrincipalKind,
    pub status: PrincipalStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PrincipalEntry {
    pub principal_id: String,
    pub name: String,
    pub kind: PrincipalKind,
    pub status: PrincipalStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

pub trait PrincipalRepository: Send {
    fn insert(
        &mut self,
        principal: &PrincipalEntry,
    ) -> impl Future<Output = Result<String, DbError>> + Send;
    fn get_by_principal_id(
        &mut self,
        principal_id: &str,
        status: PrincipalStatus,
    ) -> impl Future<Output = Result<Option<Principal>, DbError>> + Send;
    fn get_active_by_key(
        &mut self,
        key: &str,
        status: PrincipalStatus,
    ) -> impl Future<Output = Result<Option<Principal>, DbError>> + Send;
    fn get_active_by_token(
        &mut self,
        token: &str,
    ) -> impl Future<Output = Result<Option<Principal>, DbError>> + Send;
}

pub trait MetaRepository: Send {
    fn get(&mut self, key: &str) -> impl Future<Output = Result<Option<String>, DbError>> + Send;
    fn set(&mut self, key: &str, value: &str) -> impl Future<Output = Result<(), DbError>> + Send;
}

#[derive(Debug)]
pub struct AuditEntry {
    pub raft_index: Option<i64>,
    pub timestamp: i64,

    pub user_id: Option<i64>,
    pub principal_id: Option<String>,
    pub ip: Option<String>,

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

#[derive(Debug, Clone)]
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

pub struct ApiKeyEntry {
    pub user_id: i64,
    pub api_key_hash: String,
    pub api_key_prefix: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub last_used_at: Option<i64>,
}

pub trait ApiKeyRepository: Send {
    fn insert(&mut self, params: &ApiKeyEntry)
    -> impl Future<Output = Result<i64, DbError>> + Send;
}

#[derive(Debug, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: i64,
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
    fn get_by_api_key(
        &mut self,
        api_key_hash: &str,
    ) -> impl Future<Output = Result<Option<User>, DbError>> + Send;
    fn get_admin_by_api_key(
        &mut self,
        api_key_hash: &str,
    ) -> impl Future<Output = Result<Option<User>, DbError>> + Send;
}

pub struct App {
    pub app_id: String,
    pub app_name: String,
    pub created_at: i64,
    pub updated_at: i64,
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
    fn get_by_app_id(
        &mut self,
        app_id: &str,
    ) -> impl Future<Output = Result<Option<App>, DbError>> + Send;
    fn get_by_app_name(
        &mut self,
        app_name: &str,
    ) -> impl Future<Output = Result<Option<App>, DbError>> + Send;
}

#[derive(Debug, Clone, Serialize)]
pub struct SecretMapping {
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
    ) -> impl Future<Output = Result<String, DbError>> + Send;
    fn delete(
        &mut self,
        app_id: &str,
        secret_name: &str,
    ) -> impl Future<Output = Result<u64, DbError>> + Send;

    fn get_by_app_id_and_secret_name(
        &mut self,
        app_id: &str,
        secret_name: &str,
    ) -> impl Future<Output = Result<Option<SecretMapping>, DbError>> + Send;

    fn get_by_app_id_after(
        &mut self,
        app_id: &str,
        after_secret_name: Option<&str>,
        limit: i64,
    ) -> impl Future<Output = Result<Vec<SecretMapping>, DbError>> + Send;

    fn update_taint(
        &mut self,
        app_id: &str,
        secret_name: &str,
        taint: i16,
    ) -> impl Future<Output = Result<u64, DbError>> + Send;
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginConfig {
    pub backend_name: String,
    pub secret_blob: Vec<u8>,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct PluginConfigEntry {
    pub backend_name: String,
    pub secret_blob: Vec<u8>,
    pub updated_at: i64,
}

pub trait PluginConfigRepository: Send {
    fn insert(
        &mut self,
        plugin_config: &PluginConfigEntry,
    ) -> impl Future<Output = Result<String, DbError>> + Send;
    fn insert_many(
        &mut self,
        plugin_configs: Vec<PluginConfigEntry>,
    ) -> impl Future<Output = Result<Vec<String>, DbError>> + Send;
    fn get_by_backend(
        &mut self,
        backend_name: &str,
    ) -> impl Future<Output = Result<Option<PluginConfig>, DbError>> + Send;
    fn update_credentials(
        &mut self,
        backend_name: &str,
        credentials: &[u8],
        updated_at: i64,
    ) -> impl Future<Output = Result<u64, DbError>> + Send;
}
