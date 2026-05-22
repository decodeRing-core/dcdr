use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::{PluginConfigEntry, PluginConfigRepository};
use crate::request::AppRequest;
use crate::response::{AppResponse, CreatePluginConfigResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePluginConfig {
    pub backend_name: String,
    pub credentials: Vec<u8>,
    pub updated_at: i64,
}

impl From<CreatePluginConfig> for PluginConfigEntry {
    fn from(c: CreatePluginConfig) -> Self {
        Self {
            backend_name: c.backend_name,
            secret_blob: c.credentials,
            updated_at: c.updated_at,
        }
    }
}

impl From<PluginConfigEntry> for CreatePluginConfigResponse {
    fn from(e: PluginConfigEntry) -> Self {
        Self {
            backend_name: e.backend_name,
        }
    }
}

impl CreatePluginConfig {
    pub fn new(backend_name: String, credentials: Vec<u8>, updated_at: i64) -> Self {
        Self {
            backend_name,
            credentials,
            updated_at,
        }
    }

    pub fn request(backend_name: String, credentials: Vec<u8>, updated_at: i64) -> AppRequest {
        let plugin_config = Self::new(backend_name, credentials, updated_at);
        AppRequest::CreatePluginConfig(plugin_config)
    }
}

impl Action for CreatePluginConfig {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::ApiKeyCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let entry: PluginConfigEntry = self.into();
        let name = tx.plugin_config().insert(&entry).await?;
        let response = entry.into();
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreatePluginConfig(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::Plugin(name)),
        })
    }
}
