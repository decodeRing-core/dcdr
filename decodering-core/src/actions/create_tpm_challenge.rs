use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::{TpmChallengeEntry, TpmChallengeRepository};
use crate::response::{AppResponse, CreateTpmChallengeResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateTpmChallenge {
    pub actor: Actor,
    pub challenge_id: String,
    pub nonce: Vec<u8>,
    pub ek_pubkey_hash: Option<String>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
}

impl From<CreateTpmChallenge> for TpmChallengeEntry {
    fn from(c: CreateTpmChallenge) -> Self {
        Self {
            challenge_id: c.challenge_id,
            nonce: c.nonce,
            ek_pubkey_hash: c.ek_pubkey_hash,
            issued_at: c.issued_at,
            expires_at: c.expires_at,
            consumed_at: c.consumed_at,
        }
    }
}

impl From<TpmChallengeEntry> for CreateTpmChallengeResponse {
    fn from(e: TpmChallengeEntry) -> Self {
        Self {
            challenge_id: e.challenge_id,
            nonce: e.nonce,
            ek_pubkey_hash: e.ek_pubkey_hash,
            issued_at: e.issued_at,
            expires_at: e.expires_at,
            consumed_at: e.consumed_at,
        }
    }
}

impl Action for CreateTpmChallenge {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::TpmChallengeCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let tpm_challenge_entry = self.into();
        let challenge_id = tx.tpm_challenge().insert(&tpm_challenge_entry).await?;
        let tpm_challenge_response = tpm_challenge_entry.into();
        let after = serde_json::json!(tpm_challenge_response);
        let app_response = AppResponse::CreateTpmChallenge(tpm_challenge_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::TpmChallenge(challenge_id)),
        })
    }
}
