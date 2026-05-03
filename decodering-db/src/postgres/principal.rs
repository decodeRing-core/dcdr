use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::DbError;
use crate::repository::PrincipalEntry;
use crate::repository::PrincipalRepository;

pub struct PostgresPrincipalRepository<'a, 'c> {
    pub tx: &'a mut Transaction<'c, Postgres>,
}

impl<'a, 'c> PrincipalRepository for PostgresPrincipalRepository<'a, 'c> {
    async fn insert(&mut self, params: &PrincipalEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principals (name, app_id, kind, status, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(&params.name)
        .bind(&params.app_id)
        .bind(params.kind.as_str())
        .bind(params.status.as_str())
        .bind(params.created_at)
        .bind(params.updated_at)
        .fetch_one(&mut **self.tx)
        .await?;
        Ok(id)
    }
}
