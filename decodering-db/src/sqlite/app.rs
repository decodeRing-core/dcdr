use decodering_core::error::DbError;
use decodering_core::repository::AppEntry;
use decodering_core::repository::AppRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqliteAppRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> AppRepository for SqliteAppRepository<'a> {
    async fn insert(&mut self, params: &AppEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO applications (app_id, app_name, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING app_id",
        )
        .bind(&params.app_id)
        .bind(&params.app_name)
        .bind(params.created_at)
        .bind(params.updated_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
