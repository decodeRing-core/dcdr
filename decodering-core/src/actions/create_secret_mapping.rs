use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::{SecretMappingEntry, SecretMappingRespository};
use crate::response::{AppResponse, CreateSecretMappingResponse};
use crate::tx::Tx;

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

impl From<CreateSecretMapping> for SecretMappingEntry {
    fn from(c: CreateSecretMapping) -> Self {
        Self {
            app_id: c.app_id,
            secret_name: c.secret_name,
            backend: c.backend,
            mount_path: c.mount_path,
            tainted: c.tainted,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

impl From<SecretMappingEntry> for CreateSecretMappingResponse {
    fn from(e: SecretMappingEntry) -> Self {
        Self {
            app_id: e.app_id,
            secret_name: e.secret_name,
            backend: e.backend,
            mount_path: e.mount_path,
            tainted: e.tainted,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
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
        let entry = self.into();
        let id = tx.secret_mapping().insert(&entry).await?;
        let response: CreateSecretMappingResponse = entry.into();
        let secret_name = response.secret_name.clone();
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreateSecretMapping(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::SecretMapping(id, secret_name)),
        })
    }
}
