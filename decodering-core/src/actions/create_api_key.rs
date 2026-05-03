use decodering_db::Tx;
use decodering_db::repository::{ApiKeysEntry, ApiKeysRepository};
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::now_ts;
use crate::request::AppRequest;
use crate::response::{AppResponse, CreateApiKeyResponse};

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateApiKey {
    pub user_id: i64,
    pub api_key: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl CreateApiKey {
    pub fn new(user_id: i64, api_key: String, expires_at: Option<i64>) -> Self {
        Self {
            user_id,
            api_key,
            created_at: now_ts(),
            expires_at,
        }
    }

    pub fn init(api_key: String, expires_at: Option<i64>) -> Self {
        Self {
            user_id: 0, // This is set by the initialize_app action
            api_key,
            created_at: now_ts(),
            expires_at,
        }
    }

    pub fn request(user_id: i64, api_key: String, expires_at: Option<i64>) -> AppRequest {
        let api_key = CreateApiKey::new(user_id, api_key, expires_at);
        AppRequest::CreateApiKey(api_key)
    }
}

impl Action for CreateApiKey {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::User {
                user_id: self.user_id,
            },
            action_kind: ActionKind::ApiKeyCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let entry = ApiKeysEntry {
            user_id: self.user_id,
            api_key: self.api_key.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        let id = tx.api_key().insert(&entry).await?;
        let response = CreateApiKeyResponse {
            user_id: self.user_id,
            api_key: self.api_key,
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreateApiKey(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::ApiKey(id)),
        })
    }
}
