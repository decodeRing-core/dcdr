use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::UserRepository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct DeleteUser {
    pub actor: Actor,
    pub user_id: i64,
}

impl DeleteUser {
    pub fn request(actor: Actor, user_id: i64) -> AppRequest {
        let delete_secret = Self { actor, user_id };
        AppRequest::DeleteUser(delete_secret)
    }
}

impl Action for DeleteUser {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::UserDelete,
            revertible: false,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let existing = tx.user().get_by_id(self.user_id).await?;
        let rows_deleted = tx.user().delete(self.user_id).await?;
        let response = rows_deleted > 0;
        let after = serde_json::json!(response);
        let before = serde_json::json!(existing);
        let app_response = AppResponse::DeleteUser(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before),
            after_state: Some(after),
            target: Some(Target::User(self.user_id)),
        })
    }
}
