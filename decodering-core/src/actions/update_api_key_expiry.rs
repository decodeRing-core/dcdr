use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::ApiKeyRepository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct UpdateApiKeyExpiry {
    pub actor: Actor,
    pub id: i64,
    pub expiry_at: Option<i64>,
}

impl UpdateApiKeyExpiry {
    pub fn new(actor: Actor, id: i64, expiry_at: Option<i64>) -> Self {
        Self {
            actor,
            id,
            expiry_at,
        }
    }

    pub fn request(actor: Actor, id: i64, expiry_at: Option<i64>) -> AppRequest {
        let api_key = Self::new(actor, id, expiry_at);
        AppRequest::UpdateApiKeyExpiry(api_key)
    }
}

impl Action for UpdateApiKeyExpiry {
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
        let rows_updated = tx.api_key().update_expiry(self.id, self.expiry_at).await?;
        let after = serde_json::json!(&self.expiry_at);
        let app_response = AppResponse::UpdateApiKeyExpiry(rows_updated > 0);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::ApiKey(self.id)),
        })
    }
}
