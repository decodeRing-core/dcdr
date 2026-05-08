use decodering_core::error::DbError;
use decodering_core::repository::PrincipalCredentialEntry;
use decodering_core::repository::PrincipalCredentialRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqlitePrincipalCredentialRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl PrincipalCredentialRepository for SqlitePrincipalCredentialRepository<'_> {
    async fn insert(&mut self, params: &PrincipalCredentialEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_credentials (
                credential_id, principal_id, kind, lookup_key, secret_material,
                status, expires_at, last_used_at, created_at, revoked_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING credential_id",
        )
        .bind(&params.credential_id)
        .bind(&params.principal_id)
        .bind(params.kind.as_str())
        .bind(&params.lookup_key)
        .bind(&params.secret_material)
        .bind(params.status.as_str())
        .bind(params.expires_at)
        .bind(params.last_used_at)
        .bind(params.created_at)
        .bind(params.revoked_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
