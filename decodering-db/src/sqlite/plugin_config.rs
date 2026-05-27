use decodering_core::error::DbError;
use decodering_core::repository::PluginConfig;
use decodering_core::repository::PluginConfigEntry;
use decodering_core::repository::PluginConfigRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::PluginConfigRow;

pub struct SqlitePluginConfigRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl PluginConfigRepository for SqlitePluginConfigRepository<'_> {
    async fn get_by_backend(
        &mut self,
        backend_name: &str,
    ) -> Result<Option<PluginConfig>, DbError> {
        let plugin_config: Option<PluginConfigRow> = sqlx::query_as::<_, PluginConfigRow>(
            "SELECT backend_name, secret_blob, updated_at FROM plugin_configs WHERE backend_name = ? LIMIT 1",
        )
        .bind(backend_name)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(plugin_config.map(Into::into))
    }

    async fn insert(&mut self, plugin_config: &PluginConfigEntry) -> Result<String, DbError> {
        let name: String = sqlx::query_scalar(
            "INSERT INTO plugin_configs (backend_name, secret_blob, updated_at)
                VALUES (?, ?, ?) RETURNING backend_name",
        )
        .bind(&plugin_config.backend_name)
        .bind(&plugin_config.secret_blob)
        .bind(plugin_config.updated_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(name)
    }

    async fn insert_many(
        &mut self,
        plugin_configs: Vec<PluginConfigEntry>,
    ) -> Result<Vec<String>, DbError> {
        let mut names = Vec::with_capacity(plugin_configs.len());
        for p in &plugin_configs {
            sqlx::query(
                "INSERT INTO plugin_configs (backend_name, secret_blob, updated_at)
                 VALUES (?, ?, ?)",
            )
            .bind(&p.backend_name)
            .bind(&p.secret_blob)
            .bind(p.updated_at)
            .execute(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
            names.push(p.backend_name.clone());
        }
        Ok(names)
    }

    async fn update_credentials(
        &mut self,
        backend_name: &str,
        credentials: &[u8],
        updated_at: i64,
    ) -> Result<u64, DbError> {
        let result = sqlx::query(
            "UPDATE plugin_configs
             SET secret_blob = ?, updated_at = ?
             WHERE backend_name = ?",
        )
        .bind(credentials)
        .bind(updated_at)
        .bind(backend_name)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected())
    }
}
