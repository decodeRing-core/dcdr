use decodering_core::error::DbError;
use decodering_core::repository::App;
use decodering_core::repository::AppEntry;
use decodering_core::repository::AppRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::AppRow;

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

    async fn get_by_app_id(&mut self, app_id: &str) -> Result<Option<App>, DbError> {
        let app: Option<AppRow> = sqlx::query_as::<_, AppRow>(
            "SELECT app_id, app_name, created_at, updated_at FROM applications WHERE app_id = ? LIMIT 1",
        )
        .bind(app_id)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(app.map(Into::into))
    }

    async fn get_by_app_name(&mut self, app_name: &str) -> Result<Option<App>, DbError> {
        let app: Option<AppRow> = sqlx::query_as::<_, AppRow>(
            "SELECT app_id, app_name, created_at, updated_at FROM applications WHERE app_name = ? LIMIT 1",
        )
        .bind(app_name)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(app.map(Into::into))
    }
}
