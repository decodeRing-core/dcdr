use decodering_core::repository::{App, SecretMapping, Shamir, User};

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
    pub total_shares: i16,
    pub threshold: i16,
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
