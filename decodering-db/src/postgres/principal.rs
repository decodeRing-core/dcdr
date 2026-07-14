use decodering_core::domain::PrincipalStatus;
use decodering_core::error::DbError;
use decodering_core::repository::Principal;
use decodering_core::repository::PrincipalEntry;
use decodering_core::repository::PrincipalItem;
use decodering_core::repository::PrincipalRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::PrincipalItemRow;
use crate::repository::PrincipalRow;

pub struct PostgresPrincipalRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl PrincipalRepository for PostgresPrincipalRepository<'_> {
    async fn insert(&mut self, params: &PrincipalEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principals (principal_id, name, kind, status, created_at, updated_at, deleted_at) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING principal_id",
        )
        .bind(&params.principal_id)
        .bind(&params.name)
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

    async fn get_by_principal_id(
        &mut self,
        principal_id: &str,
        status: PrincipalStatus,
    ) -> Result<Option<Principal>, DbError> {
        let principal: Option<PrincipalRow> = sqlx::query_as::<_, PrincipalRow>(
            "SELECT pc.credential_id, p.principal_id, p.name, p.kind, p.status, p.created_at, p.updated_at, p.deleted_at
                FROM principals p
                INNER JOIN principal_credentials pc ON pc.principal_id = p.principal_id
                WHERE p.principal_id = $1
                  AND pc.status = $2
                  AND p.status = $2
                  AND p.deleted_at IS NULL
                  AND (pc.expires_at IS NULL OR pc.expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT)
                  AND pc.revoked_at IS NULL",
        )
        .bind(principal_id)
        .bind(status.as_str())
        .bind(status.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(principal.map(Into::into))
    }

    async fn get_active_by_key(
        &mut self,
        key_hash: &str,
        status: PrincipalStatus,
    ) -> Result<Option<Principal>, DbError> {
        let principal: Option<PrincipalRow> = sqlx::query_as::<_, PrincipalRow>(
            "SELECT pc.credential_id, p.principal_id, p.name, p.kind, p.status, p.created_at, p.updated_at, p.deleted_at
            FROM principals p
            INNER JOIN principal_credentials pc ON pc.principal_id = p.principal_id
            WHERE pc.lookup_key = $1
                AND pc.status = $2
                AND p.status = $2
                AND p.deleted_at IS NULL
                AND (pc.expires_at IS NULL OR pc.expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT)
                AND pc.revoked_at IS NULL",
        )
        .bind(key_hash)
        .bind(status.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(principal.map(Into::into))
    }

    async fn get_active_by_token(
        &mut self,
        token_hash: &str,
    ) -> Result<Option<Principal>, DbError> {
        let principal: Option<PrincipalRow> = sqlx::query_as::<_, PrincipalRow>(
            "SELECT pc.credential_id, p.principal_id, p.name, p.kind, p.status, p.created_at, p.updated_at, p.deleted_at
            FROM principal_tokens t
            INNER JOIN principals p ON p.principal_id = t.principal_id
            INNER JOIN principal_credentials pc ON pc.credential_id = t.credential_id
            WHERE t.token_hash = $1
            AND t.revoked_at IS NULL
            AND t.expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT
            AND p.status = 'active'
            AND p.deleted_at IS NULL
            AND pc.status = 'active'
            AND pc.revoked_at IS NULL",
        )
        .bind(token_hash)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(principal.map(Into::into))
    }

    async fn list(&mut self, limit: i64, offset: i64) -> Result<Vec<PrincipalItem>, DbError> {
        let rows = sqlx::query_as::<_, PrincipalItemRow>(
            "SELECT principal_id, name, kind, status, created_at, updated_at, deleted_at \
             FROM principals WHERE deleted_at IS NULL ORDER BY name LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count(&mut self) -> Result<i64, DbError> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM principals WHERE deleted_at IS NULL",
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }

    async fn get_by_id(&mut self, principal_id: &str) -> Result<Option<PrincipalItem>, DbError> {
        let row = sqlx::query_as::<_, PrincipalItemRow>(
            "SELECT principal_id, name, kind, status, created_at, updated_at, deleted_at \
             FROM principals WHERE principal_id = $1",
        )
        .bind(principal_id)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(row.map(Into::into))
    }
}
