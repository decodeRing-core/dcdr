use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::now_ts;
use crate::repository::{UserEntry, UserRepository};
use crate::request::AppRequest;
use crate::response::{AppResponse, CreateUserResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_admin: u8,
    pub created_at: i64,
}

impl CreateUser {
    pub fn new(username: &str, email: &str, password_hash: &str, is_admin: u8) -> CreateUser {
        CreateUser {
            username: username.to_string(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            is_admin,
            created_at: now_ts(),
        }
    }
    pub fn request(username: &str, email: &str, password_hash: &str, is_admin: u8) -> AppRequest {
        let user = CreateUser::new(username, email, password_hash, is_admin);
        AppRequest::CreateUser(user)
    }
}

impl Action for CreateUser {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::UserCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let entry = UserEntry {
            username: self.username.clone(),
            email: self.email.clone(),
            password_hash: self.password_hash.clone(),
            is_admin: self.is_admin == 1,
            created_at: self.created_at,
        };
        let id = tx.user().insert(&entry).await?;
        let response = CreateUserResponse {
            id: id,
            username: self.username,
            email: self.email,
            is_admin: self.is_admin,
            created_at: self.created_at,
        };
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreateUser(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::User(id)),
        })
    }
}
