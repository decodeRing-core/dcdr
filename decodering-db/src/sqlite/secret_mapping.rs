use decodering_core::error::DbError;
use decodering_core::repository::SecretMapping;
use decodering_core::repository::SecretMappingEntry;
use decodering_core::repository::SecretMappingRespository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::SecretMappingRow;

pub struct SqliteSecretMappingRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> SecretMappingRespository for SqliteSecretMappingRepository<'a> {
    async fn insert(&mut self, params: &SecretMappingEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO secret_backend_mapping (app_id, secret_name, backend, mount_path, tainted, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&params.app_id)
        .bind(&params.secret_name)
        .bind(&params.backend)
        .bind(&params.mount_path)
        .bind(params.tainted)
        .bind(params.created_at)
        .bind(params.updated_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }

    async fn get_by_app_id_secret_name(
        &mut self,
        app_id: &str,
        secret_name: &str,
    ) -> Result<Option<SecretMapping>, DbError> {
        let secret_mapping: Option<SecretMappingRow> = sqlx::query_as::<_, SecretMappingRow>(
            "SELECT app_id, secret_name, backend, mount_path, tainted, created_at, updated_at
                FROM secret_backend_mapping
                WHERE app_id = ? AND secret_name = ?",
        )
        .bind(&app_id)
        .bind(&secret_name)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(secret_mapping.map(Into::into))
    }
}
