use std::str::FromStr;

use decodering_core::domain::{
    AuditOutcome, PrincipalCredentialKind, PrincipalKind, PrincipalStatus,
};
use decodering_core::repository::{
    App, Audit, AuthChallenge, PluginConfig, Principal, PrincipalAppGrant, PrincipalCredential,
    SecretMapping, Shamir, User,
};

#[derive(sqlx::FromRow)]
pub struct PrincipalAppGrantRow {
    pub principal_id: String,
    pub app_id: String,
    pub granted_at: i64,
    pub granted_by: Option<i64>,
    pub revoked_at: Option<i64>,
    pub revoked_by: Option<i64>,
}

impl From<PrincipalAppGrantRow> for PrincipalAppGrant {
    fn from(r: PrincipalAppGrantRow) -> Self {
        Self {
            principal_id: r.principal_id,
            app_id: r.app_id,
            granted_at: r.granted_at,
            granted_by: r.granted_by,
            revoked_at: r.revoked_at,
            revoked_by: r.revoked_by,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct SecretMappingRow {
    app_id: String,
    secret_name: String,
    backend: String,
    mount_path: String,
    tainted: i16,
    created_at: i64,
    updated_at: i64,
}

impl From<SecretMappingRow> for SecretMapping {
    fn from(r: SecretMappingRow) -> Self {
        Self {
            app_id: r.app_id,
            secret_name: r.secret_name,
            backend: r.backend,
            mount_path: r.mount_path,
            tainted: r.tainted,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct ShamirRow {
    pub id: i64,
    pub total_shares: i32,
    pub threshold: i32,
    pub validation_hash: Vec<u8>,
    pub created_at: i64,
}

impl From<ShamirRow> for Shamir {
    fn from(r: ShamirRow) -> Self {
        Self {
            id: r.id,
            total_shares: r.total_shares,
            threshold: r.threshold,
            validation_hash: r.validation_hash,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct UserRow {
    id: i64,
    username: String,
    email: String,
    password_hash: String,
    is_admin: bool,
    created_at: i64,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        Self {
            id: r.id,
            username: r.username,
            email: r.email,
            password_hash: r.password_hash,
            is_admin: r.is_admin,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct AppRow {
    app_id: String,
    app_name: String,
    created_at: i64,
    updated_at: i64,
}

impl From<AppRow> for App {
    fn from(a: AppRow) -> Self {
        Self {
            app_id: a.app_id,
            app_name: a.app_name,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}
#[derive(sqlx::FromRow)]
pub struct PrincipalRow {
    pub credential_id: String,
    pub principal_id: String,
    pub name: String,
    pub kind: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

impl From<PrincipalRow> for Principal {
    fn from(r: PrincipalRow) -> Self {
        Self {
            credential_id: r.credential_id,
            principal_id: r.principal_id,
            name: r.name,
            kind: PrincipalKind::from_str(r.kind.as_str()).unwrap_or(PrincipalKind::Human),
            status: PrincipalStatus::from_str(r.status.as_str())
                .unwrap_or(PrincipalStatus::Disabled),
            deleted_at: r.deleted_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct AuthChallengeRow {
    challenge_id: String,
    method: String,
    payload: Vec<u8>,
    issued_at: i64,
    expires_at: i64,
    consumed_at: Option<i64>,
}

impl From<AuthChallengeRow> for AuthChallenge {
    fn from(r: AuthChallengeRow) -> Self {
        Self {
            challenge_id: r.challenge_id,
            method: r.method,
            payload: r.payload,
            issued_at: r.issued_at,
            expires_at: r.expires_at,
            consumed_at: r.consumed_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct PrincipalCredentialRow {
    pub credential_id: String,
    pub principal_id: String,
    pub kind: String,
    pub lookup_key: String,
    pub secret_material: String,
    pub status: String,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

impl From<PrincipalCredentialRow> for PrincipalCredential {
    fn from(r: PrincipalCredentialRow) -> Self {
        Self {
            credential_id: r.credential_id,
            principal_id: r.principal_id,
            kind: PrincipalCredentialKind::from_str(&r.kind)
                .unwrap_or(PrincipalCredentialKind::ApiKey),
            lookup_key: r.lookup_key,
            secret_material: r.secret_material,
            status: PrincipalStatus::from_str(&r.status).unwrap_or(PrincipalStatus::Disabled),
            expires_at: r.expires_at,
            last_used_at: r.last_used_at,
            created_at: r.created_at,
            revoked_at: r.revoked_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct PluginConfigRow {
    pub backend_name: String,
    pub secret_blob: Vec<u8>,
    pub updated_at: i64,
}

impl From<PluginConfigRow> for PluginConfig {
    fn from(r: PluginConfigRow) -> Self {
        Self {
            backend_name: r.backend_name,
            secret_blob: r.secret_blob,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct AuditRow {
    pub id: i64,
    pub raft_index: Option<i64>,
    pub timestamp: i64,

    pub user_id: Option<i64>,
    pub principal_id: Option<String>,
    pub ip: Option<String>,

    pub action_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,

    pub outcome: String,
    pub reason: Option<String>,

    pub before_state: Option<String>,
    pub after_state: Option<String>,
    pub metadata: Option<String>,

    pub revertible: bool,
    pub undone_by: Option<i64>,
    pub undoes: Option<i64>,
    pub actor_username: Option<String>,
}

impl From<AuditRow> for Audit {
    fn from(r: AuditRow) -> Self {
        Self {
            id: r.id,
            raft_index: r.raft_index,
            timestamp: r.timestamp,
            user_id: r.user_id,
            principal_id: r.principal_id,
            ip: r.ip,
            action_type: r.action_type,
            target_type: r.target_type,
            target_id: r.target_id,
            outcome: AuditOutcome::from_str(&r.outcome).unwrap_or(AuditOutcome::Error),
            reason: r.reason,
            before_state: r.before_state,
            after_state: r.after_state,
            metadata: r.metadata,
            revertible: r.revertible,
            undone_by: r.undone_by,
            undoes: r.undoes,
            actor_username: r.actor_username,
        }
    }
}
