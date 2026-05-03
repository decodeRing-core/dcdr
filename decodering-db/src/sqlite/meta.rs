use sqlx::{Sqlite, Transaction};

use crate::{DbError, repository::MetaRepository};

pub struct SqliteMetaRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> MetaRepository for SqliteMetaRepository<'a> {
    async fn get(&mut self, key: &str) -> Result<Option<String>, DbError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM meta WHERE key = ?")
            .bind(key)
            .fetch_optional(&mut **self.tx)
            .await?;
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
        .await?;
        Ok(())
    }
}
