use decodering_core::error::DbError;
use decodering_core::repository::MetaRepository;
use sqlx::{Sqlite, Transaction};

use crate::error::map_sqlx;

pub struct SqliteMetaRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl MetaRepository for SqliteMetaRepository<'_> {
    async fn get(&mut self, key: &str) -> Result<Option<String>, DbError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM meta WHERE key = ?")
            .bind(key)
            .fetch_optional(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(row.map(|(v,)| v))
    }

    async fn set(&mut self, key: &str, value: &str) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO meta (key, value) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }
}
