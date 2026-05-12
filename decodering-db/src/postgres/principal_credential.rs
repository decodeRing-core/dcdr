use decodering_core::domain::PrincipalCredentialKind;
use decodering_core::error::DbError;
use decodering_core::repository::PrincipalCredential;
use decodering_core::repository::PrincipalCredentialEntry;
use decodering_core::repository::PrincipalCredentialRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::PrincipalCredentialRow;

pub struct PostgresPrincipalCredentialRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl PrincipalCredentialRepository for PostgresPrincipalCredentialRepository<'_> {
    async fn insert(&mut self, params: &PrincipalCredentialEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_credentials (
                credential_id, principal_id, kind, lookup_key, secret_material,
                status, expires_at, last_used_at, created_at, revoked_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING credential_id",
        )
        .bind(&params.credential_id)
        .bind(&params.principal_id)
        .bind(params.kind.as_str())
        .bind(&params.lookup_key)
        .bind(&params.secret_material)
        .bind(params.status.as_str())
        .bind(params.expires_at)
        .bind(params.last_used_at)
        .bind(params.created_at)
        .bind(params.revoked_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }

    async fn get_active_by_kind_and_lookup_key(
        &mut self,
        kind: PrincipalCredentialKind,
        lookup_key: &str,
    ) -> Result<Option<PrincipalCredential>, DbError> {
        let principal_credential: Option<PrincipalCredentialRow> =
            sqlx::query_as::<_, PrincipalCredentialRow>(
                "SELECT pc.credential_id,
                    pc.principal_id,
                    pc.kind,
                    pc.lookup_key,
                    pc.secret_material,
                    pc.status,
                    pc.expires_at,
                    pc.last_used_at,
                    pc.created_at,
                    pc.revoked_at
               FROM principal_credentials pc
               INNER JOIN principals p ON p.principal_id = pc.principal_id
              WHERE pc.kind = $1
                AND pc.lookup_key = $2
                AND pc.status = 'active'
                AND pc.revoked_at IS NULL
                AND (pc.expires_at IS NULL OR pc.expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT)
                AND p.status = 'active'
                AND p.deleted_at IS NULL",
            )
            .bind(kind.as_str())
            .bind(lookup_key)
            .fetch_optional(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(principal_credential.map(Into::into))
    }
}
