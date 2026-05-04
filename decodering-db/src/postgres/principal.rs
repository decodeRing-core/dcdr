use decodering_core::error::DbError;
use decodering_core::repository::PrincipalEntry;
use decodering_core::repository::PrincipalRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct PostgresPrincipalRepository<'a, 'c> {
    pub tx: &'a mut Transaction<'c, Postgres>,
}

impl<'a, 'c> PrincipalRepository for PostgresPrincipalRepository<'a, 'c> {
    async fn insert(&mut self, params: &PrincipalEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principals (principal_id, name, app_id, kind, status, created_at, updated_at, deleted_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING principal_id",
        )
        .bind(&params.principal_id)
        .bind(&params.name)
        .bind(&params.app_id)
        .bind(params.kind.as_str())
        .bind(params.status.as_str())
        .bind(params.created_at)
        .bind(params.updated_at)
        .bind(params.deleted_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
