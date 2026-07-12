use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::domain::PrincipalStatus;
use crate::error::ExecutionError;
use crate::repository::PrincipalCredentialRepository;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct UpdatePrincipalCredentialStatus {
    pub actor: Actor,
    pub credential_id: String,
    pub principal_id: String,
    pub status: PrincipalStatus,
    pub revoked_at: Option<i64>,
}

impl Action for UpdatePrincipalCredentialStatus {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::PrincipalCredentialUpdate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let principal_credential = tx
            .principal_credential()
            .get_by_credential_and_principal(&self.credential_id, &self.principal_id)
            .await?;
        let before_state = serde_json::json!(principal_credential);
        let _ = tx
            .principal_credential()
            .update_status(&self.credential_id, self.status, self.revoked_at)
            .await?;
        let response = self.status;
        let after = serde_json::json!(response);
        let app_response = AppResponse::UpdatePrincipalCredentialStatus(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before_state),
            after_state: Some(after),
            target: Some(Target::PrincipalCredential(self.credential_id)),
        })
    }
}
