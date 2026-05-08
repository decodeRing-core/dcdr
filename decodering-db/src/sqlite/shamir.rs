use decodering_core::error::DbError;
use decodering_core::repository::Shamir;
use decodering_core::repository::ShamirEntry;
use decodering_core::repository::ShamirRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::ShamirRow;

pub struct SqliteShamirRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl ShamirRepository for SqliteShamirRepository<'_> {
    async fn get_first(&mut self) -> Result<Option<Shamir>, DbError> {
        let shamir: Option<ShamirRow> = sqlx::query_as::<_, ShamirRow>(
            "SELECT id, total_shares, threshold, validation_hash, created_at FROM shamir_configuration ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(shamir.map(Into::into))
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
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
