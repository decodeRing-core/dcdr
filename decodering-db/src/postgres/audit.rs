use decodering_core::error::DbError;
use decodering_core::repository::AuditEntry;
use decodering_core::repository::AuditRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct PostgresAuditRepository<'a, 'c> {
    pub tx: &'a mut Transaction<'c, Postgres>,
}

impl<'a, 'c> AuditRepository for PostgresAuditRepository<'a, 'c> {
    async fn insert(&mut self, params: &AuditEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO audit_log
                (
                raft_index,
                timestamp,
                user_id,
                principal_id,
                action_type,
                target_type,
                target_id,
                outcome,
                reason,
                before_state,
                after_state,
                metadata,
                revertible,
                undone_by,
                undoes)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NULL, $14)
                RETURNING id",
        )
        .bind(params.raft_index)
        .bind(params.timestamp)
        .bind(params.user_id)
        .bind(&params.principal_id)
        .bind(&params.action_type)
        .bind(&params.target_type)
        .bind(&params.target_id)
        .bind(params.outcome.as_str())
        .bind(&params.reason)
        .bind(&params.before_state)
        .bind(&params.after_state)
        .bind(&params.metadata)
        .bind(params.revertible)
        .bind(params.undoes)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
