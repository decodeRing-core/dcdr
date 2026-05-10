use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::{ApiKeyEntry, ApiKeyRepository};
use crate::request::AppRequest;
use crate::response::{AppResponse, CreateApiKeyResponse};
use crate::time::now_ts;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateApiKey {
    pub user_id: i64,
    pub api_key_hash: String,
    pub api_key_prefix: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
    pub last_used_at: Option<i64>,
}

impl From<CreateApiKey> for ApiKeyEntry {
    fn from(c: CreateApiKey) -> Self {
        Self {
            user_id: c.user_id,
            api_key_hash: c.api_key_hash,
            api_key_prefix: c.api_key_prefix,
            created_at: c.created_at,
            expires_at: c.expires_at,
            revoked_at: c.revoked_at,
            last_used_at: c.last_used_at,
        }
    }
}

impl From<ApiKeyEntry> for CreateApiKeyResponse {
    fn from(e: ApiKeyEntry) -> Self {
        Self {
            user_id: e.user_id,
            created_at: e.created_at,
            expires_at: e.expires_at,
        }
    }
}

impl CreateApiKey {
    pub fn new(
        user_id: i64,
        api_key_hash: String,
        api_key_prefix: String,
        expires_at: Option<i64>,
    ) -> Self {
        Self {
            user_id,
            created_at: now_ts(),
            expires_at,
            api_key_hash,
            api_key_prefix,
            revoked_at: None,
            last_used_at: None,
        }
    }

    pub fn init(api_key_hash: String, api_key_prefix: String, expires_at: Option<i64>) -> Self {
        Self {
            user_id: 0, // This is set by the initialize_app action
            created_at: now_ts(),
            expires_at,
            api_key_hash,
            api_key_prefix,
            revoked_at: None,
            last_used_at: None,
        }
    }

    pub fn request(
        user_id: i64,
        api_key_hash: String,
        api_key_prefix: String,
        expires_at: Option<i64>,
    ) -> AppRequest {
        let api_key = Self::new(user_id, api_key_hash, api_key_prefix, expires_at);
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
        let entry = self.into();
        let id = tx.api_key().insert(&entry).await?;
        let response = entry.into();
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
