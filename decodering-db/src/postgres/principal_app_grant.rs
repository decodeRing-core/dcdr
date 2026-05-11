use crate::error::map_sqlx;
use decodering_core::error::DbError;
use decodering_core::repository::{PrincipalAppGrantEntry, PrincipalAppGrantRepository};
use sqlx::{Postgres, Transaction};

pub struct PostgresPrincipalAppGrantRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl PrincipalAppGrantRepository for PostgresPrincipalAppGrantRepository<'_> {
    async fn insert_many(&mut self, params: &[PrincipalAppGrantEntry]) -> Result<(), DbError> {
        let principal_ids: Vec<&str> = params.iter().map(|p| p.principal_id.as_str()).collect();
        let app_ids: Vec<&str> = params.iter().map(|p| p.app_id.as_str()).collect();
        let granted_at: Vec<i64> = params.iter().map(|p| p.granted_at).collect();
        let granted_by: Vec<Option<i64>> = params.iter().map(|p| p.granted_by).collect();
        let revoked_at: Vec<Option<i64>> = params.iter().map(|p| p.revoked_at).collect();
        let revoked_by: Vec<Option<i64>> = params.iter().map(|p| p.revoked_by).collect();

        sqlx::query(
            "INSERT INTO principal_app_grants \
             (principal_id, app_id, granted_at, granted_by, revoked_at, revoked_by) \
             SELECT * FROM UNNEST($1::text[], $2::text[], $3::bigint[], $4::bigint[], $5::bigint[], $6::bigint[])",
        )
        .bind(&principal_ids)
        .bind(&app_ids)
        .bind(&granted_at)
        .bind(&granted_by)
        .bind(&revoked_at)
        .bind(&revoked_by)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;

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
