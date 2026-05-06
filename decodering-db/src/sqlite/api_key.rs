use decodering_core::error::DbError;
use decodering_core::repository::ApiKeyEntry;
use decodering_core::repository::ApiKeyRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqliteApiKeysRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> ApiKeyRepository for SqliteApiKeysRepository<'a> {
    async fn insert(&mut self, params: &ApiKeyEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO api_keys (user_id, api_key_hash, api_key_prefix, created_at, expires_at, revoked_at, last_used_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
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
}
