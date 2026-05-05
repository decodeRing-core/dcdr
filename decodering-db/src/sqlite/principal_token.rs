use decodering_core::error::DbError;
use decodering_core::repository::PrincipalTokenEntry;
use decodering_core::repository::PrincipalTokenRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqlitePrincipalTokenRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> PrincipalTokenRepository for SqlitePrincipalTokenRepository<'a> {
    async fn insert(&mut self, params: &PrincipalTokenEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_token (
                token_id, token_hash, principal_id, credential_id, issued_at,
                expires_at, revoked_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING credential_id",
        )
        .bind(&params.token_id)
        .bind(&params.token_hash)
        .bind(&params.principal_id)
        .bind(&params.credential_id)
        .bind(&params.issued_at)
        .bind(&params.expires_at)
        .bind(&params.revoked_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
