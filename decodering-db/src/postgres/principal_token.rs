use decodering_core::error::DbError;
use decodering_core::repository::PrincipalTokenEntry;
use decodering_core::repository::PrincipalTokenRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct PostgresPrincipalTokenRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl<'a> PrincipalTokenRepository for PostgresPrincipalTokenRepository<'a> {
    async fn insert(&mut self, params: &PrincipalTokenEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_tokens (
                token_id, token_hash, principal_id, credential_id, issued_at,
                expires_at, revoked_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING credential_id",
        )
        .bind(&params.token_id)
        .bind(&params.token_hash)
        .bind(&params.principal_id)
        .bind(&params.credential_id)
        .bind(params.issued_at)
        .bind(params.expires_at)
        .bind(params.revoked_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
