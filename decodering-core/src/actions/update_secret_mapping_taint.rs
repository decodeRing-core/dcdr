use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::SecretMappingRespository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct UpdateSecretMappingTaint {
    pub secret_name: String,
    pub app_id: String,
    pub taint: bool,
}

impl UpdateSecretMappingTaint {
    pub fn request(
        app_id: impl Into<String>,
        secret_name: impl Into<String>,
        taint: bool,
    ) -> AppRequest {
        let taint_secret = Self {
            app_id: app_id.into(),
            secret_name: secret_name.into(),
            taint,
        };
        AppRequest::UpdateSecretMappingTaint(taint_secret)
    }
}

impl Action for UpdateSecretMappingTaint {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::SecretMappingTaint,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let secret_mapping = tx
            .secret_mapping()
            .get_by_app_id_and_secret_name(&self.app_id, &self.secret_name)
            .await?;
        let before_state = serde_json::json!(secret_mapping);
        let _ = tx
            .secret_mapping()
            .update_taint(&self.app_id, &self.secret_name, i16::from(self.taint))
            .await?;
        let response = self.taint;
        let after = serde_json::json!(response);
        let app_response = AppResponse::UpdateSecretMappingTaint(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before_state),
            after_state: Some(after),
            target: Some(Target::SecretMapping(self.app_id, self.secret_name)),
        })
    }
}
