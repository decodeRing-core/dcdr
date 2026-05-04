use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::domain::{PrincipalKind, PrincipalStatus};
use crate::error::ExecutionError;
use crate::repository::{PrincipalEntry, PrincipalRepository};
use crate::response::{AppResponse, CreatePrincipalResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipal {
    pub principal_id: String,
    pub name: String,
    pub app_id: String,
    pub kind: PrincipalKind,
    pub status: PrincipalStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

impl Action for CreatePrincipal {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::PrincipalCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let principal_entry = PrincipalEntry {
            principal_id: self.principal_id,
            name: self.name,
            app_id: self.app_id,
            kind: self.kind,
            status: self.status,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        };
        let principal_id = tx.principal().insert(&principal_entry).await?;
        let principal_response = CreatePrincipalResponse {
            principal_id: principal_id.clone(),
            name: principal_entry.name,
            app_id: principal_entry.app_id,
            kind: principal_entry.kind,
            status: principal_entry.status,
            created_at: principal_entry.created_at,
            updated_at: principal_entry.updated_at,
            deleted_at: principal_entry.deleted_at,
        };
        let after = serde_json::json!(principal_response);
        let app_response = AppResponse::CreatePrincipal(principal_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::Principal(principal_id)),
        })
    }
}
