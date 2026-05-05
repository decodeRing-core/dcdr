use decodering_core::error::DbError;
use decodering_core::repository::ApiKeyEntry;
use decodering_core::repository::ApiKeyRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct PostgresApiKeysRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl<'a> ApiKeyRepository for PostgresApiKeysRepository<'a> {
    async fn insert(&mut self, params: &ApiKeyEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO api_keys (user_id, api_key, created_at, expires_at) VALUES ($1, $2, $3, $4) RETURNING id",
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
