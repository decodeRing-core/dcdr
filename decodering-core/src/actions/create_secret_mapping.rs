use decodering_db::Tx;
use decodering_db::repository::{SecretMappingEntry, SecretMappingRespository};
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::response::{AppResponse, CreateSecretMappingResponse};

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateSecretMapping {
    pub app_id: String,
    pub secret_name: String,
    pub backend: String,
    pub mount_path: String,
    pub tainted: i16,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Action for CreateSecretMapping {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::SecretMappingCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let entry = SecretMappingEntry {
            app_id: self.app_id.clone(),
            secret_name: self.secret_name.clone(),
            backend: self.backend.clone(),
            mount_path: self.mount_path.clone(),
            tainted: self.tainted,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        let id = tx.secret_mapping().insert(&entry).await?;
        let response = CreateSecretMappingResponse {
            app_id: self.app_id,
            secret_name: self.secret_name,
            backend: self.backend,
            mount_path: self.mount_path,
            tainted: self.tainted,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreateSecretMapping(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::SecretMapping(id, entry.secret_name)),
        })
    }
}
