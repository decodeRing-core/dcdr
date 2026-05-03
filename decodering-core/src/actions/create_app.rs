use decodering_db::Tx;
use decodering_db::repository::{AppEntry, AppRepository};
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::now_ts;
use crate::request::AppRequest;
use crate::response::{AppResponse, CreateAppResponse};

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateApp {
    pub app_id: String,
    pub app_name: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl CreateApp {
    pub fn request(app_id: String, app_name: String) -> AppRequest {
        let app = CreateApp {
            app_id,
            app_name,
            created_at: now_ts(),
            updated_at: now_ts(),
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
        let entry = AppEntry {
            app_id: self.app_id.clone(),
            app_name: self.app_name.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        let id = tx.app().insert(&entry).await?;
        let response = CreateAppResponse {
            app_id: self.app_id,
            app_name: self.app_name,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
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
