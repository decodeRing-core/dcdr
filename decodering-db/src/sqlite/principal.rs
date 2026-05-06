use decodering_core::error::DbError;
use decodering_core::repository::PrincipalEntry;
use decodering_core::repository::PrincipalRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqlitePrincipalRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> PrincipalRepository for SqlitePrincipalRepository<'a> {
    async fn insert(&mut self, params: &PrincipalEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principals (principal_id, name, app_id, kind, status, created_at, updated_at, deleted_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING principal_id",
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
