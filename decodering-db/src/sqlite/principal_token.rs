use decodering_core::error::DbError;
use decodering_core::repository::PrincipalToken;
use decodering_core::repository::PrincipalTokenEntry;
use decodering_core::repository::PrincipalTokenRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::PrincipalTokenRow;

pub struct SqlitePrincipalTokenRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl PrincipalTokenRepository for SqlitePrincipalTokenRepository<'_> {
    async fn insert(&mut self, params: &PrincipalTokenEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_tokens (
                token_id, token_hash, principal_id, credential_id, issued_at,
                expires_at, revoked_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
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

    async fn list_by_principal(
        &mut self,
        principal_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PrincipalToken>, DbError> {
        let rows = sqlx::query_as::<_, PrincipalTokenRow>(
            "SELECT token_id, token_hash, principal_id, credential_id, issued_at, expires_at, revoked_at \
             FROM principal_tokens WHERE principal_id = ? ORDER BY issued_at DESC LIMIT ? OFFSET ?",
        )
        .bind(principal_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_by_principal(&mut self, principal_id: &str) -> Result<i64, DbError> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM principal_tokens WHERE principal_id = ?",
        )
        .bind(principal_id)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }
}
