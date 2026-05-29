use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::PrincipalAppGrantRepository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct DeletePrincipalAppGrant {
    pub actor: Actor,
    pub principal_id: String,
    pub app_id: String,
}

impl DeletePrincipalAppGrant {
    pub fn request(
        actor: Actor,
        app_id: impl Into<String>,
        principal_id: impl Into<String>,
    ) -> AppRequest {
        let delete_secret = Self {
            actor,
            app_id: app_id.into(),
            principal_id: principal_id.into(),
        };
        AppRequest::DeletePrincipalAppGrant(delete_secret)
    }
}

impl Action for DeletePrincipalAppGrant {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::SecretMappingDelete,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let existing = tx
            .principal_app_grant()
            .get_by_app_id_and_principal_id(&self.app_id, &self.principal_id)
            .await?;
        let rows_deleted = tx
            .principal_app_grant()
            .delete(&self.app_id, &self.principal_id)
            .await?;
        let response = rows_deleted > 0;
        let after = serde_json::json!(response);
        let before = serde_json::json!(existing);
        let app_response = AppResponse::DeletePrincipalAppGrant(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before),
            after_state: Some(after),
            target: Some(Target::PrincipalAppGrant(Some(self.app_id))),
        })
    }
}
