use decodering_core::error::DbError;
use decodering_core::repository::PrincipalToken;
use decodering_core::repository::PrincipalTokenEntry;
use decodering_core::repository::PrincipalTokenRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::PrincipalTokenRow;

pub struct PostgresPrincipalTokenRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl PrincipalTokenRepository for PostgresPrincipalTokenRepository<'_> {
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

    async fn list_by_principal(
        &mut self,
        principal_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PrincipalToken>, DbError> {
        let rows = sqlx::query_as::<_, PrincipalTokenRow>(
            "SELECT token_id, token_hash, principal_id, credential_id, issued_at, expires_at, revoked_at \
             FROM principal_tokens WHERE principal_id = ? ORDER BY issued_at DESC LIMIT $1 OFFSET $2",
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
            "SELECT COUNT(*) FROM principal_tokens WHERE principal_id = $1",
        )
        .bind(principal_id)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }

    async fn revoke(&mut self, token_id: &str, revoked_at: i64) -> Result<u64, DbError> {
        let result = sqlx::query(
            "UPDATE principal_tokens SET revoked_at = $1 WHERE token_id = $2 AND revoked_at IS NULL",
        )
        .bind(revoked_at)
        .bind(token_id)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected())
    }
}
