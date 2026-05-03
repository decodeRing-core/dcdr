use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::DbError;
use crate::repository::AppEntry;
use crate::repository::AppRepository;

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
        .await?;
        Ok(id)
    }
}
