use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::SecretMappingRespository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct DeleteSecretMapping {
    pub app_id: String,
    pub secret_name: String,
}

impl DeleteSecretMapping {
    pub fn request(app_id: impl Into<String>, secret_name: impl Into<String>) -> AppRequest {
        let delete_secret = Self {
            app_id: app_id.into(),
            secret_name: secret_name.into(),
        };
        AppRequest::DeleteSecretMapping(delete_secret)
    }
}

impl Action for DeleteSecretMapping {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::SecretMappingDelete,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let existing = tx
            .secret_mapping()
            .get_by_app_id_secret_name(&self.app_id, &self.secret_name)
            .await?;
        let rows_deleted = tx
            .secret_mapping()
            .delete(&self.app_id, &self.secret_name)
            .await?;
        let response = rows_deleted > 0;
        let after = serde_json::json!(response);
        let before = serde_json::json!(existing);
        let app_response = AppResponse::DeleteSecretMapping(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before),
            after_state: Some(after),
            target: Some(Target::SecretMapping(self.app_id, self.secret_name)),
        })
    }
}
