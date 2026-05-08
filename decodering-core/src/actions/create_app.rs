use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::now_ts;
use crate::repository::{AppEntry, AppRepository};
use crate::request::AppRequest;
use crate::response::{AppResponse, CreateAppResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateApp {
    pub app_id: String,
    pub app_name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<CreateApp> for AppEntry {
    fn from(c: CreateApp) -> Self {
        Self {
            app_id: c.app_id,
            app_name: c.app_name,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

impl From<AppEntry> for CreateAppResponse {
    fn from(e: AppEntry) -> Self {
        Self {
            app_id: e.app_id,
            app_name: e.app_name,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

impl CreateApp {
    pub fn request(app_id: String, app_name: String) -> AppRequest {
        let timestamp = now_ts();
        let app = CreateApp {
            app_id,
            app_name,
            created_at: timestamp,
            updated_at: timestamp,
        };
        AppRequest::CreateApp(app)
    }
}

impl Action for CreateApp {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::AppCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let entry = self.into();
        let id = tx.app().insert(&entry).await?;
        let response = entry.into();
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreateApp(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::App(id)),
        })
    }
}
