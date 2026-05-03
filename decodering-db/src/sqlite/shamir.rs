use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::DbError;
use crate::repository::Shamir;
use crate::repository::ShamirEntry;
use crate::repository::ShamirRepository;

pub struct SqliteShamirRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> ShamirRepository for SqliteShamirRepository<'a> {
    async fn get_first(&mut self) -> Result<Option<Shamir>, DbError> {
        let shamir = sqlx::query_as::<_, Shamir>(
            "SELECT id, total_shares, threshold, validation_hash, created_at FROM shamir_configuration ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&mut **self.tx)
        .await?;
        Ok(shamir)
    }

    async fn insert(&mut self, params: &ShamirEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO shamir_configuration (total_shares, threshold, validation_hash, created_at) VALUES (?, ?, ?, ?) RETURNING id",
        )
        .bind(params.total_shares)
        .bind(params.threshold)
        .bind(&params.validation_hash)
        .bind(params.created_at)
        .fetch_one(&mut **self.tx) // or &pool
        .await?;
        Ok(id)
    }
}
