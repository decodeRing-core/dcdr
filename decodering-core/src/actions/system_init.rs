use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::actions::create_api_key::CreateApiKey;
use crate::actions::create_shamir_configuration::CreateShamirConfiguration;
use crate::actions::create_user::CreateUser;
use crate::audit::ActionKind;
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::{ApiKeyEntry, ApiKeyRepository, ShamirRepository, UserRepository};
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::response::SystemInitResponse;
use crate::tx::Tx;

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
