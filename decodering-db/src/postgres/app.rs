use decodering_core::error::DbError;
use decodering_core::repository::App;
use decodering_core::repository::AppEntry;
use decodering_core::repository::AppRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::AppRow;

pub struct PostgresAppRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl AppRepository for PostgresAppRepository<'_> {
    async fn insert(&mut self, params: &AppEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO applications (app_id, app_name, created_at, updated_at) VALUES ($1, $2, $3, $4) RETURNING app_id",
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
            "SELECT app_id, app_name, created_at, updated_at FROM applications WHERE app_id = $1 LIMIT 1",
        )
        .bind(app_id)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(app.map(Into::into))
    }

    async fn get_by_app_name(&mut self, app_name: &str) -> Result<Option<App>, DbError> {
        let app: Option<AppRow> = sqlx::query_as::<_, AppRow>(
            "SELECT app_id, app_name, created_at, updated_at FROM applications WHERE app_name = $1 LIMIT 1",
        )
        .bind(app_name)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(app.map(Into::into))
    }

    async fn list(&mut self, limit: i64, offset: i64) -> Result<Vec<App>, DbError> {
        let rows = sqlx::query_as::<_, AppRow>(
            "SELECT app_id, app_name, created_at, updated_at FROM applications ORDER BY app_name LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count(&mut self) -> Result<i64, DbError> {
        let n = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM applications")
            .fetch_one(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(n)
    }

    async fn get_by_id(&mut self, app_id: &str) -> Result<Option<App>, DbError> {
        let row = sqlx::query_as::<_, AppRow>(
            "SELECT app_id, app_name, created_at, updated_at FROM applications WHERE app_id = $1",
        )
        .bind(app_id)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Into::into))
    }
}
