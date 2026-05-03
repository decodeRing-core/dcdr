use decodering_db::Tx;
use decodering_db::repository::ApiKeysEntry;
use decodering_db::repository::ApiKeysRepository;
use decodering_db::repository::ShamirEntry;
use decodering_db::repository::ShamirRepository;
use decodering_db::repository::UserEntry;
use decodering_db::repository::UserRepository;
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::actions::create_api_key::CreateApiKey;
use crate::actions::create_shamir_configuration::CreateShamirConfiguration;
use crate::actions::create_user::CreateUser;
use crate::audit::ActionKind;
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::response::CreateApiKeyResponse;
use crate::response::CreateShamirConfigurationResponse;
use crate::response::CreateUserResponse;
use crate::response::SystemInitResponse;

#[derive(Serialize, Debug, Deserialize)]
pub struct SystemInit {
    pub shamir: CreateShamirConfiguration,
    pub user: CreateUser,
    pub api_key: CreateApiKey,
}

impl SystemInit {
    pub fn request(
        shamir: CreateShamirConfiguration,
        user: CreateUser,
        api_key: CreateApiKey,
    ) -> AppRequest {
        let app = SystemInit {
            shamir,
            user,
            api_key,
        };
        AppRequest::SystemInit(app)
    }
}

impl Action for SystemInit {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::SystemInit,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let shamir_entry = ShamirEntry {
            total_shares: self.shamir.total_shares,
            threshold: self.shamir.threshold,
            validation_hash: self.shamir.validation_hash,
            created_at: self.shamir.timestamp,
        };
        let shamir_id = tx.shamir().insert(&shamir_entry).await?;
        let shamir_response = CreateShamirConfigurationResponse {
            id: shamir_id,
            total_shares: shamir_entry.total_shares,
            threshold: shamir_entry.threshold,
            validation_hash: shamir_entry.validation_hash,
            timestamp: shamir_entry.created_at,
        };

        let user_entry = UserEntry {
            username: self.user.username,
            email: self.user.email,
            password_hash: self.user.password_hash,
            is_admin: self.user.is_admin == 1,
            created_at: self.user.created_at,
        };
        let user_id = tx.user().insert(&user_entry).await?;
        let user_response = CreateUserResponse {
            id: user_id,
            username: user_entry.username,
            email: user_entry.email,
            is_admin: self.user.is_admin,
            created_at: user_entry.created_at,
        };

        let api_key_entry = ApiKeysEntry {
            user_id,
            api_key: self.api_key.api_key,
            created_at: self.api_key.created_at,
            expires_at: self.api_key.expires_at,
        };
        let _ = tx.api_key().insert(&api_key_entry).await?;
        let api_key_response = CreateApiKeyResponse {
            user_id,
            api_key: api_key_entry.api_key,
            created_at: api_key_entry.created_at,
            expires_at: api_key_entry.expires_at,
        };

        let response = SystemInitResponse {
            shamir: shamir_response,
            user: user_response,
            api_key: api_key_response,
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
