use decodering_core::error::DbError;
use decodering_core::repository::TpmChallenge;
use decodering_core::repository::TpmChallengeEntry;
use decodering_core::repository::TpmChallengeRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::TpmChallengeRow;

pub struct PostgresTpmChallengeRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl TpmChallengeRepository for PostgresTpmChallengeRepository<'_> {
    async fn insert(&mut self, params: &TpmChallengeEntry) -> Result<String, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO tpm_challenge (
                challenge_id, nonce, ek_pubkey_hash, issued_at, expires_at,
                consumed_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING challenge_id",
        )
        .bind(&params.challenge_id)
        .bind(&params.nonce)
        .bind(&params.ek_pubkey_hash)
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
            "UPDATE tpm_challenge
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
            "DELETE FROM tpm_challenges
              WHERE expires_at <= EXTRACT(EPOCH FROM NOW())::BIGINT",
        )
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(result.rows_affected())
    }

    async fn get_active(&mut self, challenge_id: &str) -> Result<Option<TpmChallenge>, DbError> {
        let tpm_challenge = sqlx::query_as::<_, TpmChallengeRow>(
            "SELECT challenge_id, nonce, ek_pubkey_hash, issued_at, expires_at, consumed_at
               FROM tpm_challenges
              WHERE challenge_id = $1
                AND consumed_at IS NULL
                AND expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT",
        )
        .bind(challenge_id)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;

        Ok(tpm_challenge.map(Into::into))
    }
}
