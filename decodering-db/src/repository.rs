use decodering_core::repository::{SecretMapping, Shamir};

#[derive(sqlx::FromRow)]
pub struct SecretMappingRow {
    id: i64,
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
            id: r.id,
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
