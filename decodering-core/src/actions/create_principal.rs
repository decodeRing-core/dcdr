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
    pub actor: Actor,
    pub principal_id: String,
    pub name: String,
    pub kind: PrincipalKind,
    pub status: PrincipalStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

impl From<CreatePrincipal> for PrincipalEntry {
    fn from(c: CreatePrincipal) -> Self {
        Self {
            principal_id: c.principal_id,
            name: c.name,
            kind: c.kind,
            status: c.status,
            created_at: c.created_at,
            updated_at: c.updated_at,
            deleted_at: c.deleted_at,
        }
    }
}

impl From<PrincipalEntry> for CreatePrincipalResponse {
    fn from(e: PrincipalEntry) -> Self {
        Self {
            principal_id: e.principal_id,
            name: e.name,
            kind: e.kind,
            status: e.status,
            created_at: e.created_at,
            updated_at: e.updated_at,
            deleted_at: e.deleted_at,
        }
    }
}

impl Action for CreatePrincipal {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::PrincipalCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let principal_entry: PrincipalEntry = self.into();
        let principal_id = tx.principal().insert(&principal_entry).await?;
        let principal_response: CreatePrincipalResponse = principal_entry.into();
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
