use decodering_core::domain::PrincipalStatus;
use decodering_core::error::DbError;
use decodering_core::repository::Principal;
use decodering_core::repository::PrincipalEntry;
use decodering_core::repository::PrincipalRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::PrincipalRow;

pub struct PostgresPrincipalRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl<'a> PrincipalRepository for PostgresPrincipalRepository<'a> {
    async fn insert(&mut self, params: &PrincipalEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principals (principal_id, name, app_id, kind, status, created_at, updated_at, deleted_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING principal_id",
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

    async fn get_by_app_id_and_key(
        &mut self,
        app_id: &str,
        key_hash: &str,
        status: PrincipalStatus,
    ) -> Result<Option<Principal>, DbError> {
        let principal: Option<PrincipalRow> = sqlx::query_as::<_, PrincipalRow>(
            "SELECT p.principal_id, p.name, p.app_id, p.kind, p.status, p.created_at, p.updated_at, p.deleted_at
                FROM principals p
                INNER JOIN principal_credentials pc ON pc.principal_id = p.principal_id
                WHERE p.app_id = $1
                  AND pc.lookup_key = $2
                  AND pc.status = $3
                  AND p.status = $3
                  AND p.deleted_at IS NULL
                  AND (pc.expires_at IS NULL OR pc.expires_at > unixepoch())
                  AND pc.revoked_at IS NULL",
        )
        .bind(app_id)
        .bind(key_hash)
        .bind(status.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(principal.map(Into::into))
    }
}
