use decodering_core::error::DbError;
use decodering_core::repository::AuthChallenge;
use decodering_core::repository::AuthChallengeEntry;
use decodering_core::repository::AuthChallengeRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::AuthChallengeRow;

pub struct PostgresAuthChallengeRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl AuthChallengeRepository for PostgresAuthChallengeRepository<'_> {
    async fn insert(&mut self, params: &AuthChallengeEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO auth_challenges (
                challenge_id, method, payload, issued_at, expires_at,
                consumed_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING challenge_id",
        )
        .bind(&params.challenge_id)
        .bind(&params.method)
        .bind(&params.payload)
        .bind(params.issued_at)
        .bind(params.expires_at)
        .bind(params.consumed_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }

    async fn update_consumed(
        &mut self,
        challenge_id: &str,
        consumed_at: i64,
    ) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "UPDATE auth_challenges
             SET consumed_at = $1
             WHERE challenge_id = $2
             RETURNING challenge_id",
        )
        .bind(consumed_at)
        .bind(challenge_id)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }

    async fn delete_expired(&mut self) -> Result<u64, DbError> {
        let result = sqlx::query(
            "DELETE FROM auth_challenges
              WHERE expires_at <= EXTRACT(EPOCH FROM NOW())::BIGINT",
        )
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected())
    }

    async fn get_active(&mut self, challenge_id: &str) -> Result<Option<AuthChallenge>, DbError> {
        let auth_challenge = sqlx::query_as::<_, AuthChallengeRow>(
            "SELECT challenge_id, method, payload, issued_at, expires_at, consumed_at
               FROM auth_challenges
              WHERE challenge_id = $1
                AND consumed_at IS NULL
                AND expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT",
        )
        .bind(challenge_id)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;

        Ok(auth_challenge.map(Into::into))
    }
}
