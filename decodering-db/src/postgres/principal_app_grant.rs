use crate::error::map_sqlx;
use crate::repository::PrincipalAppGrantRow;
use decodering_core::error::DbError;
use decodering_core::repository::PrincipalAppGrant;
use decodering_core::repository::PrincipalAppGrantEntry;
use decodering_core::repository::PrincipalAppGrantRepository;
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

    async fn delete(&mut self, app_id: &str, principal_id: &str) -> Result<u64, DbError> {
        let rows_affected = sqlx::query(
            "DELETE FROM principal_app_grants
             WHERE app_id = $1 AND principal_id = $2",
        )
        .bind(app_id)
        .bind(principal_id)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        Ok(rows_affected)
    }

    async fn get_by_app_id_and_principal_id(
        &mut self,
        app_id: &str,
        principal_id: &str,
    ) -> Result<Option<PrincipalAppGrant>, DbError> {
        let principal_app_grant: Option<PrincipalAppGrantRow> =
            sqlx::query_as::<_, PrincipalAppGrantRow>(
                "SELECT principal_id, app_id, granted_at, granted_by, revoked_at, revoked_by
                FROM principal_app_grants
                WHERE app_id = $1 AND principal_id = $2",
            )
            .bind(app_id)
            .bind(principal_id)
            .fetch_optional(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(principal_app_grant.map(Into::into))
    }

    #[allow(clippy::option_if_let_else)]
    async fn get_by_principal_id_after(
        &mut self,
        principal_id: &str,
        after_app_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<PrincipalAppGrant>, DbError> {
        let rows: Vec<PrincipalAppGrantRow> = match after_app_id {
            Some(cursor) => sqlx::query_as::<_, PrincipalAppGrantRow>(
                "SELECT principal_id, app_id, granted_at, granted_by, revoked_at, revoked_by
                 FROM principal_app_grants
                 WHERE principal_id = $1 AND app_id > $2
                 LIMIT $3",
            )
            .bind(principal_id)
            .bind(cursor)
            .bind(limit),
            None => sqlx::query_as::<_, PrincipalAppGrantRow>(
                "SELECT principal_id, app_id, granted_at, granted_by, revoked_at, revoked_by
                 FROM principal_app_grants
                 WHERE principal_id = $1
                 LIMIT $2",
            )
            .bind(principal_id)
            .bind(limit),
        }
        .fetch_all(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_by_principal(
        &mut self,
        principal_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PrincipalAppGrant>, DbError> {
        let rows = sqlx::query_as::<_, PrincipalAppGrantRow>(
            "SELECT g.principal_id, g.app_id, a.app_name, g.granted_at, g.granted_by, g.revoked_at \
             FROM principal_app_grants g LEFT JOIN applications a ON a.app_id = g.app_id \
             WHERE g.principal_id = ? ORDER BY g.granted_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(principal_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_by_principal(&mut self, principal_id: &str) -> Result<i64, DbError> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM principal_app_grants WHERE principal_id = $1",
        )
        .bind(principal_id)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }
}
