use decodering_core::error::DbError;
use decodering_core::repository::ApiKey;
use decodering_core::repository::ApiKeyEntry;
use decodering_core::repository::ApiKeyRepository;
use decodering_core::repository::ApiKeyUser;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::ApiKeyRow;
use crate::repository::ApiKeyUserRow;

pub struct PostgresApiKeysRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl ApiKeyRepository for PostgresApiKeysRepository<'_> {
    async fn get_by_id(&mut self, id: i64) -> Result<Option<ApiKey>, DbError> {
        let user: Option<ApiKeyRow> = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, user_id, key_hash, key_prefix, created_at, expires_at, revoked_at, last_used_at
            FROM api_keys
            WHERE api_keys.id = $1",
        )
        .bind(id)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(user.map(Into::into))
    }

    async fn insert(&mut self, params: &ApiKeyEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO api_keys (user_id, key_hash, key_prefix, created_at, expires_at, revoked_at, last_used_at) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        )
        .bind(params.user_id)
        .bind(&params.api_key_hash)
        .bind(&params.api_key_prefix)
        .bind(params.created_at)
        .bind(params.expires_at)
        .bind(params.revoked_at)
        .bind(params.last_used_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }

    async fn list(&mut self, limit: i64, offset: i64) -> Result<Vec<ApiKeyUser>, DbError> {
        let rows = sqlx::query_as::<_, ApiKeyUserRow>(
            "SELECT k.id, k.user_id, u.username, u.email, k.key_prefix, k.created_at, \
                    k.expires_at, k.revoked_at, k.last_used_at \
             FROM api_keys k LEFT JOIN users u ON u.id = k.user_id \
             ORDER BY k.id DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count(&mut self) -> Result<i64, DbError> {
        let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM api_keys")
            .fetch_one(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(total)
    }

    async fn revoke(&mut self, id: i64, revoked_at: i64) -> Result<u64, DbError> {
        let rows =
            sqlx::query("UPDATE api_keys SET revoked_at = $1 WHERE id = $2 AND revoked_at IS NULL")
                .bind(revoked_at)
                .bind(id)
                .execute(&mut **self.tx)
                .await
                .map_err(map_sqlx)?
                .rows_affected();
        Ok(rows)
    }

    async fn update_expiry(&mut self, id: i64, expires_at: Option<i64>) -> Result<u64, DbError> {
        let rows = sqlx::query("UPDATE api_keys SET expires_at = $1 WHERE id = $2")
            .bind(expires_at)
            .bind(id)
            .execute(&mut **self.tx)
            .await
            .map_err(map_sqlx)?
            .rows_affected();
        Ok(rows)
    }

    async fn delete(&mut self, id: i64) -> Result<u64, DbError> {
        let rows = sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(id)
            .execute(&mut **self.tx)
            .await
            .map_err(map_sqlx)?
            .rows_affected();
        Ok(rows)
    }
}
