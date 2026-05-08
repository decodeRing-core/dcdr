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

impl From<CreateUser> for UserEntry {
    fn from(c: CreateUser) -> Self {
        Self {
            username: c.username,
            email: c.email,
            password_hash: c.password_hash,
            is_admin: c.is_admin == 1,
            created_at: c.created_at,
        }
    }
}

impl From<UserEntry> for CreateUserResponse {
    fn from(e: UserEntry) -> Self {
        Self {
            username: e.username,
            email: e.email,
            is_admin: e.is_admin,
            created_at: e.created_at,
        }
    }
}

impl CreateUser {
    pub fn new(username: &str, email: &str, password_hash: &str, is_admin: u8) -> Self {
        Self {
            username: username.to_owned(),
            email: email.to_owned(),
            password_hash: password_hash.to_owned(),
            is_admin,
            created_at: now_ts(),
        }
    }
    pub fn request(username: &str, email: &str, password_hash: &str, is_admin: u8) -> AppRequest {
        let user = Self::new(username, email, password_hash, is_admin);
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
        let entry = self.into();
        let id = tx.user().insert(&entry).await?;
        let response = entry.into();
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
