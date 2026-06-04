use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::AuthChallengeRepository;
use crate::response::{AppResponse, ConsumeAuthChallengeResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct UpdateAuthChallengeConsumedAt {
    pub actor: Actor,
    pub challenge_id: String,
    pub consumed_at: i64,
}

impl Action for UpdateAuthChallengeConsumedAt {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::AuthChallengeConsume,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let challenge = tx.auth_challenge().get_active(&self.challenge_id).await?;
        let before_state = serde_json::json!(challenge);
        let challenge_id = tx
            .auth_challenge()
            .update_consumed(&self.challenge_id, self.consumed_at)
            .await?;
        let auth_challenge_response = ConsumeAuthChallengeResponse {
            challenge_id: challenge_id.clone(),
            payload: challenge.map(|f| f.payload).unwrap_or_default(),
        };
        let after = serde_json::json!(auth_challenge_response);
        let app_response = AppResponse::ConsumeAuthChallenge(auth_challenge_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before_state),
            after_state: Some(after),
            target: Some(Target::AuthChallenge(challenge_id)),
        })
    }
}
