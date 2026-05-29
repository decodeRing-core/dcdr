use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::{PrincipalAppGrantEntry, PrincipalAppGrantRepository};
use crate::response::{AppResponse, CreatePrincipalAppGrantResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalAppGrants(pub Vec<CreatePrincipalAppGrant>);

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalAppGrant {
    pub actor: Actor,
    pub principal_id: String,
    pub app_id: String,
    pub granted_at: i64,
    pub granted_by: Option<i64>,
    pub revoked_at: Option<i64>,
    pub revoked_by: Option<i64>,
}

impl From<CreatePrincipalAppGrant> for PrincipalAppGrantEntry {
    fn from(c: CreatePrincipalAppGrant) -> Self {
        Self {
            principal_id: c.principal_id,
            app_id: c.app_id,
            granted_at: c.granted_at,
            granted_by: c.granted_by,
            revoked_at: c.revoked_at,
            revoked_by: c.revoked_by,
        }
    }
}

impl From<PrincipalAppGrantEntry> for CreatePrincipalAppGrantResponse {
    fn from(e: PrincipalAppGrantEntry) -> Self {
        Self {
            principal_id: e.principal_id,
            app_id: e.app_id,
            granted_at: e.granted_at,
            granted_by: e.granted_by,
            revoked_at: e.revoked_at,
            revoked_by: e.revoked_by,
        }
    }
}

impl Action for CreatePrincipalAppGrant {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::PrincipalAppGrantCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let principal_app_grant_entry = self.into();
        let principal_id = tx
            .principal_app_grant()
            .insert(&principal_app_grant_entry)
            .await?;
        let principal_app_grant_response = principal_app_grant_entry.into();
        let after = serde_json::json!(principal_app_grant_response);
        let app_response = AppResponse::CreatePrincipalAppGrant(principal_app_grant_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::PrincipalAppGrant(Some(principal_id))),
        })
    }
}

impl Action for CreatePrincipalAppGrants {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self
                .0
                .first()
                .map_or(Actor::None { ip: None }, |f| f.actor.clone()),
            action_kind: ActionKind::PrincipalAppGrantCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let mut entries: Vec<PrincipalAppGrantEntry> = vec![];
        let mut responses: Vec<CreatePrincipalAppGrantResponse> = vec![];
        for app in self.0 {
            let entry: PrincipalAppGrantEntry = app.into();
            entries.push(entry.clone());
            responses.push(entry.into());
        }
        tx.principal_app_grant().insert_many(&entries).await?;
        let after = serde_json::json!(responses);
        let app_response = AppResponse::CreatePrincipalAppGrants(responses);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::PrincipalAppGrant(None)),
        })
    }
}
