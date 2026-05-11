use decodering_core::error::DbError;
use decodering_core::repository::PrincipalAppGrantEntry;
use decodering_core::repository::PrincipalAppGrantRepository;
use sqlx::QueryBuilder;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqlitePrincipalAppGrantRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl PrincipalAppGrantRepository for SqlitePrincipalAppGrantRepository<'_> {
    async fn insert_many(&mut self, params: &[PrincipalAppGrantEntry]) -> Result<(), DbError> {
        const CHUNK: usize = 1000; // 6 * 1000 = 6000 params, safely under the limit
        for chunk in params.chunks(CHUNK) {
            let mut qb = QueryBuilder::<Sqlite>::new(
                "INSERT INTO principal_app_grants \
                 (principal_id, app_id, granted_at, granted_by, revoked_at, revoked_by) ",
            );
            qb.push_values(chunk, |mut b, entry| {
                b.push_bind(&entry.principal_id)
                    .push_bind(&entry.app_id)
                    .push_bind(entry.granted_at)
                    .push_bind(entry.granted_by)
                    .push_bind(entry.revoked_at)
                    .push_bind(entry.revoked_by);
            });
            qb.build().execute(&mut **self.tx).await.map_err(map_sqlx)?;
        }
        Ok(())
    }

    async fn insert(&mut self, params: &PrincipalAppGrantEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_app_grants (
            principal_id, app_id, granted_at, granted_by, revoked_at, revoked_by
            ) VALUES (?, ?, ?, ?, ?, ?)
            RETURNING principal_id",
        )
        .bind(&params.principal_id)
        .bind(&params.app_id)
        .bind(params.granted_at)
        .bind(params.granted_by)
        .bind(params.revoked_at)
        .bind(params.revoked_by)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
