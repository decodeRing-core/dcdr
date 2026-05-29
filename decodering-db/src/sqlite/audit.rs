use decodering_core::error::DbError;
use decodering_core::repository::AuditEntry;
use decodering_core::repository::AuditRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqliteAuditRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl AuditRepository for SqliteAuditRepository<'_> {
    async fn insert(&mut self, params: &AuditEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO audit_log
                (
                raft_index,
                timestamp,
                user_id,
                principal_id,
                ip,
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
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?)
                RETURNING id",
        )
        .bind(params.raft_index)
        .bind(params.timestamp)
        .bind(params.user_id)
        .bind(&params.principal_id)
        .bind(&params.ip)
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
