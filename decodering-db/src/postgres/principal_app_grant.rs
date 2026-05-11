use crate::error::map_sqlx;
use decodering_core::error::DbError;
use decodering_core::repository::{PrincipalAppGrantEntry, PrincipalAppGrantRepository};
use sqlx::{Postgres, QueryBuilder, Transaction};

pub struct PostgresPrincipalAppGrantRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl PrincipalAppGrantRepository for PostgresPrincipalAppGrantRepository<'_> {
    async fn insert_many(&mut self, params: &[PrincipalAppGrantEntry]) -> Result<(), DbError> {
        const CHUNK: usize = 1000;
        for chunk in params.chunks(CHUNK) {
            let mut qb = QueryBuilder::<Postgres>::new(
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
            qb.push(
                " ON CONFLICT(principal_id, app_id) DO UPDATE SET \
                    granted_at = excluded.granted_at, \
                    granted_by = excluded.granted_by, \
                    revoked_at = excluded.revoked_at, \
                    revoked_by = excluded.revoked_by",
            );
            qb.build().execute(&mut **self.tx).await.map_err(map_sqlx)?;
        }
        Ok(())
    }

    async fn insert(&mut self, params: &PrincipalAppGrantEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_app_grants (
            principal_id, app_id, granted_at, granted_by, revoked_at, revoked_by
            ) VALUES ($1, $2, $3, $4, $5, $6)
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
