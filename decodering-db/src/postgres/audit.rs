use decodering_core::error::DbError;
use decodering_core::repository::Audit;
use decodering_core::repository::AuditEntry;
use decodering_core::repository::AuditRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::AuditRow;

pub struct PostgresAuditRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl AuditRepository for PostgresAuditRepository<'_> {
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
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NULL, $15)
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

    async fn list(&mut self, limit: i64, offset: i64) -> Result<Vec<Audit>, DbError> {
        let rows = sqlx::query_as::<_, AuditRow>(
            "SELECT a.id, a.raft_index, a.timestamp, a.user_id, a.principal_id, a.ip, \
                    a.action_type, a.target_type, a.target_id, a.outcome, a.reason, \
                    a.before_state, a.after_state, a.metadata, a.revertible, \
                    a.undone_by, a.undoes, u.username AS actor_username \
             FROM audit_log a LEFT JOIN users u ON u.id = a.user_id \
             ORDER BY a.id DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count(&mut self) -> Result<i64, DbError> {
        let n = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_log")
            .fetch_one(&mut **self.tx)
            .await
            .map_err(map_sqlx)?;
        Ok(n)
    }

    async fn count_outcomes_since(&mut self, since: i64) -> Result<(i64, i64), DbError> {
        let row: (i64, i64) = sqlx::query_as(
            "SELECT \
               COALESCE(SUM(CASE WHEN outcome = 'denied' THEN 1 ELSE 0 END), 0), \
               COALESCE(SUM(CASE WHEN outcome = 'error' THEN 1 ELSE 0 END), 0) \
             FROM audit_log WHERE timestamp >= $1",
        )
        .bind(since)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(row)
    }
}
