use decodering_core::domain::PrincipalCredentialKind;
use decodering_core::domain::PrincipalStatus;
use decodering_core::error::DbError;
use decodering_core::repository::PrincipalCredential;
use decodering_core::repository::PrincipalCredentialEntry;
use decodering_core::repository::PrincipalCredentialRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::PrincipalCredentialRow;

pub struct SqlitePrincipalCredentialRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl PrincipalCredentialRepository for SqlitePrincipalCredentialRepository<'_> {
    async fn insert(&mut self, params: &PrincipalCredentialEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO principal_credentials (
                credential_id, principal_id, kind, lookup_key, secret_material,
                status, expires_at, last_used_at, created_at, revoked_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
              WHERE pc.kind = ?
                AND pc.lookup_key = ?
                AND pc.status = 'active'
                AND pc.revoked_at IS NULL
                AND (pc.expires_at IS NULL OR pc.expires_at > unixepoch())
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

    async fn get_pending_by_kind_and_credential_and_principal(
        &mut self,
        principal_id: String,
        credential_id: String,
        kind: PrincipalCredentialKind,
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
              WHERE pc.kind = ?
                AND pc.principal_id = ?
                AND pc.credential_id = ?
                AND pc.status = 'pending'
                AND pc.revoked_at IS NULL
                AND (pc.expires_at IS NULL OR pc.expires_at > unixepoch())
                AND p.status = 'active'
                AND p.deleted_at IS NULL",
            )
            .bind(kind.as_str())
            .bind(principal_id)
            .bind(credential_id)
            .fetch_optional(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(principal_credential.map(Into::into))
    }

    async fn get_by_credential_and_principal(
        &mut self,
        credential_id: &str,
        principal_id: &str,
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
               WHERE pc.credential_id = ?
                  AND pc.principal_id = ?",
            )
            .bind(credential_id)
            .bind(principal_id)
            .fetch_optional(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(principal_credential.map(Into::into))
    }

    async fn update_last_used(
        &mut self,
        credential_id: &str,
        last_used_at: i64,
    ) -> Result<u64, DbError> {
        let result = sqlx::query(
            "UPDATE principal_credentials
             SET last_used_at = ?
             WHERE credential_id = ?",
        )
        .bind(last_used_at)
        .bind(credential_id)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected())
    }

    async fn update_status(
        &mut self,
        credential_id: &str,
        status: PrincipalStatus,
        revoked_at: Option<i64>,
    ) -> Result<u64, DbError> {
        let result = sqlx::query(
            "UPDATE principal_credentials \
             SET status = ?, revoked_at = ? \
             WHERE credential_id = ?",
        )
        .bind(status.as_str())
        .bind(revoked_at)
        .bind(credential_id)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected())
    }

    async fn list_by_principal(
        &mut self,
        principal_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PrincipalCredential>, DbError> {
        let rows = sqlx::query_as::<_, PrincipalCredentialRow>(
            "SELECT credential_id, principal_id, kind, status, expires_at, last_used_at, created_at, revoked_at \
             FROM principal_credentials WHERE principal_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(principal_id).bind(limit).bind(offset)
        .fetch_all(&mut **self.tx).await.map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_by_principal(&mut self, principal_id: &str) -> Result<i64, DbError> {
        let n = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM principal_credentials WHERE principal_id = ?",
        )
        .bind(principal_id)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(n)
    }
}
