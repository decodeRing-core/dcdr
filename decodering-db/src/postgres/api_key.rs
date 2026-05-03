use decodering_core::error::DbError;
use decodering_core::repository::ApiKeysEntry;
use decodering_core::repository::ApiKeysRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct PostgresApiKeysRepository<'a, 'c> {
    pub tx: &'a mut Transaction<'c, Postgres>,
}

impl<'a, 'c> ApiKeysRepository for PostgresApiKeysRepository<'a, 'c> {
    async fn insert(&mut self, params: &ApiKeysEntry) -> Result<i64, DbError> {
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
