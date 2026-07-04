use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::ApiKeyRepository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct DeleteApiKey {
    pub actor: Actor,
    pub id: i64,
}

impl DeleteApiKey {
    pub fn request(actor: Actor, id: i64) -> AppRequest {
        let delete_api_key = Self { actor, id };
        AppRequest::DeleteApiKey(delete_api_key)
    }
}

impl Action for DeleteApiKey {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::ApiKeyDelete,
            revertible: false,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let existing = tx.api_key().get_by_id(self.id).await?;
        let rows_deleted = tx.api_key().delete(self.id).await?;
        let response = rows_deleted > 0;
        let after = serde_json::json!(response);
        let before = serde_json::json!(existing);
        let app_response = AppResponse::DeleteUser(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before),
            after_state: Some(after),
            target: Some(Target::ApiKey(self.id)),
        })
    }
}
