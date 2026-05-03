use decodering_core::error::DbError;
use decodering_core::repository::ApiKeysEntry;
use decodering_core::repository::ApiKeysRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqliteApiKeysRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> ApiKeysRepository for SqliteApiKeysRepository<'a> {
    async fn insert(&mut self, params: &ApiKeysEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO api_keys (user_id, api_key, created_at, expires_at) VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(params.user_id)
        .bind(&params.api_key)
        .bind(params.created_at)
        .bind(params.expires_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
