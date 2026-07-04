use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::ApiKeyRepository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::time::now_ts;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct RevokeApiKey {
    pub actor: Actor,
    pub id: i64,
    pub revoked_at: i64,
}

impl RevokeApiKey {
    pub fn new(actor: Actor, id: i64) -> Self {
        let revoked_at = now_ts();
        Self {
            actor,
            id,
            revoked_at,
        }
    }
    pub fn request(actor: Actor, id: i64) -> AppRequest {
        let revoke_api_key = Self::new(actor, id);
        AppRequest::RevokeApiKey(revoke_api_key)
    }
}

impl Action for RevokeApiKey {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::ApiKeyRevoke,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let id = tx.api_key().revoke(self.id, self.revoked_at).await?;
        let response = id > 0;
        let after = serde_json::json!(response);
        let app_response = AppResponse::RevokeApiKey(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::ApiKey(self.id)),
        })
    }
}
