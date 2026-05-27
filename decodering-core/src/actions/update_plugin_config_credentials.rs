use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::{PluginConfigEntry, PluginConfigRepository};
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct UpdatePluginConfigCredentials {
    pub backend: String,
    pub credentials: Vec<u8>,
    pub updated_at: i64,
}

impl UpdatePluginConfigCredentials {
    pub fn new(backend: String, credentials: Vec<u8>, updated_at: i64) -> Self {
        Self {
            backend,
            credentials,
            updated_at,
        }
    }

    pub fn request(backend_name: String, credentials: Vec<u8>, updated_at: i64) -> AppRequest {
        let plugin_config = Self::new(backend_name, credentials, updated_at);
        AppRequest::UpdatePluginConfigCredentials(plugin_config)
    }
}

impl Action for UpdatePluginConfigCredentials {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::PluginConfigCredentialsUpdate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let plugin_config = tx.plugin_config().get_by_backend(&self.backend).await?;
        let before_state = serde_json::json!(plugin_config);
        if plugin_config.is_some() {
            tx.plugin_config()
                .update_credentials(&self.backend, &self.credentials, self.updated_at)
                .await?;
        } else {
            let plugin_config_entry = PluginConfigEntry {
                backend_name: self.backend.clone(),
                secret_blob: self.credentials.clone(),
                updated_at: self.updated_at,
            };
            tx.plugin_config().insert(&plugin_config_entry).await?;
        }
        let response = self.credentials;
        let after = serde_json::json!(response);
        let app_response = AppResponse::UpdatePluginConfigSecrets(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: Some(before_state),
            after_state: Some(after),
            target: Some(Target::Plugin(self.backend)),
        })
    }
}
