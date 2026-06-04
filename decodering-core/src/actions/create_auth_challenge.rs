use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::{AuthChallengeEntry, AuthChallengeRepository};
use crate::response::{AppResponse, CreateAuthChallengeResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateAuthChallenge {
    pub actor: Actor,
    pub challenge_id: String,
    pub method: String,
    pub payload: Vec<u8>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub consumed_at: Option<i64>,
}

impl From<CreateAuthChallenge> for AuthChallengeEntry {
    fn from(c: CreateAuthChallenge) -> Self {
        Self {
            challenge_id: c.challenge_id,
            method: c.method,
            payload: c.payload,
            issued_at: c.issued_at,
            expires_at: c.expires_at,
            consumed_at: c.consumed_at,
        }
    }
}

impl From<AuthChallengeEntry> for CreateAuthChallengeResponse {
    fn from(e: AuthChallengeEntry) -> Self {
        Self {
            challenge_id: e.challenge_id,
            method: e.method,
            payload: e.payload,
            issued_at: e.issued_at,
            expires_at: e.expires_at,
            consumed_at: e.consumed_at,
        }
    }
}

impl Action for CreateAuthChallenge {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::AuthChallengeCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let auth_challenge_entry = self.into();
        let challenge_id = tx.auth_challenge().insert(&auth_challenge_entry).await?;
        let auth_challenge_response = auth_challenge_entry.into();
        let after = serde_json::json!(auth_challenge_response);
        let app_response = AppResponse::CreateAuthChallenge(auth_challenge_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::AuthChallenge(challenge_id)),
        })
    }
}
