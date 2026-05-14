use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::TpmChallengeRepository;
use crate::response::{AppResponse, ConsumeTpmChallengeResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct UpdateTpmChallengeConsumedAt {
    pub challenge_id: String,
    pub consumed_at: i64,
}

impl Action for UpdateTpmChallengeConsumedAt {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::TpmChallengeConsume,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let challenge = tx.tpm_challenge().get_active(&self.challenge_id).await?;
        let before_state = serde_json::json!(challenge);
        let challenge_id = tx
            .tpm_challenge()
            .updated_consumed(&self.challenge_id, self.consumed_at)
            .await?;
        let tpm_challenge_response = ConsumeTpmChallengeResponse {
            challenge_id: challenge_id.clone(),
        };
        let after = serde_json::json!(tpm_challenge_response);
        let app_response = AppResponse::ConsumeTpmChallenge(tpm_challenge_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before_state),
            after_state: Some(after),
            target: Some(Target::TpmChallenge(challenge_id)),
        })
    }
}
