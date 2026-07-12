use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::PrincipalTokenRepository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::time::now_ts;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct RevokePrincipalToken {
    pub actor: Actor,
    pub id: String,
    pub revoked_at: i64,
}

impl RevokePrincipalToken {
    pub fn new(actor: Actor, id: &str) -> Self {
        let revoked_at = now_ts();
        Self {
            actor,
            id: id.to_owned(),
            revoked_at,
        }
    }
    pub fn request(actor: Actor, id: &str) -> AppRequest {
        let revoke_principal_token = Self::new(actor, id);
        AppRequest::RevokePrincipalToken(revoke_principal_token)
    }
}

impl Action for RevokePrincipalToken {
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
        let id = tx
            .principal_token()
            .revoke(&self.id, self.revoked_at)
            .await?;
        let response = id > 0;
        let after = serde_json::json!(response);
        let app_response = AppResponse::RevokePrincipalToken(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::PrincipalToken(self.id)),
        })
    }
}
