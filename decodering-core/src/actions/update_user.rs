use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::repository::UserRepository;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::time::now_ts;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct UpdateUser {
    pub actor: Actor,
    pub id: i64,
    pub email: String,
    pub password_hash: Option<String>,
    pub is_admin: u8,
    pub created_at: i64,
}

impl UpdateUser {
    pub fn new(
        actor: Actor,
        id: i64,
        email: &str,
        password_hash: Option<String>,
        is_admin: u8,
    ) -> Self {
        Self {
            actor,
            id,
            email: email.to_owned(),
            password_hash,
            is_admin,
            created_at: now_ts(),
        }
    }
    pub fn request(
        actor: Actor,
        id: i64,
        email: &str,
        password_hash: Option<String>,
        is_admin: u8,
    ) -> AppRequest {
        let user = Self::new(actor, id, email, password_hash, is_admin);
        AppRequest::UpdateUser(user)
    }
}

impl Action for UpdateUser {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::UserUpdate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let id = tx
            .user()
            .update(
                self.id,
                &self.email,
                self.is_admin == 1,
                self.password_hash.as_deref(),
            )
            .await?;
        let response = id > 0;
        let after = serde_json::json!(response);
        let app_response = AppResponse::UpdateUser(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::User(self.id)),
        })
    }
}
