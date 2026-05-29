use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::actions::create_api_key::CreateApiKey;
use crate::actions::create_plugin_config::CreatePluginConfig;
use crate::actions::create_shamir_configuration::CreateShamirConfiguration;
use crate::actions::create_user::CreateUser;
use crate::audit::ActionKind;
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::ApiKeyEntry;
use crate::repository::ApiKeyRepository;
use crate::repository::PluginConfigEntry;
use crate::repository::PluginConfigRepository;
use crate::repository::ShamirRepository;
use crate::repository::UserRepository;
use crate::request::AppRequest;
use crate::response::SystemInitResponse;
use crate::response::{AppResponse, CreatePluginConfigResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct SystemInit {
    pub actor: Actor,
    pub shamir: CreateShamirConfiguration,
    pub user: CreateUser,
    pub api_key: CreateApiKey,
    pub plugin_config: Vec<CreatePluginConfig>,
}

impl SystemInit {
    pub fn request(
        actor: Actor,
        shamir: CreateShamirConfiguration,
        user: CreateUser,
        api_key: CreateApiKey,
        plugin_config: Vec<CreatePluginConfig>,
    ) -> AppRequest {
        let app = Self {
            actor,
            shamir,
            user,
            api_key,
            plugin_config,
        };
        AppRequest::SystemInit(app)
    }
}

impl Action for SystemInit {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::SystemInit,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let shamir_entry = self.shamir.into();
        let _ = tx.shamir().insert(&shamir_entry).await?;
        let shamir_response = shamir_entry.into();

        let user_entry = self.user.into();
        let user_id = tx.user().insert(&user_entry).await?;
        let user_response = user_entry.into();

        let mut api_key_entry: ApiKeyEntry = self.api_key.into();
        api_key_entry.user_id = user_id;
        let _ = tx.api_key().insert(&api_key_entry).await?;
        let api_key_response = api_key_entry.into();

        let plugin_config_entries: Vec<PluginConfigEntry> =
            self.plugin_config.into_iter().map(Into::into).collect();
        let _ = tx
            .plugin_config()
            .insert_many(plugin_config_entries.clone())
            .await?;
        let plugin_config_response: Vec<CreatePluginConfigResponse> =
            plugin_config_entries.into_iter().map(Into::into).collect();

        let response = SystemInitResponse {
            shamir: shamir_response,
            user: user_response,
            api_key: api_key_response,
            plugin_config: plugin_config_response,
        };
        let after = serde_json::json!(response);
        let app_response = AppResponse::SystemInit(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: None,
        })
    }
}
