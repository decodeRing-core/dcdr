use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::domain::{PrincipalCredentialKind, PrincipalStatus};
use crate::error::ExecutionError;
use crate::repository::{PrincipalCredentialEntry, PrincipalCredentialRepository};
use crate::response::{AppResponse, CreatePrincipalCredentialResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalCredential {
    pub actor: Actor,
    pub credential_id: String,
    pub principal_id: String,
    pub kind: PrincipalCredentialKind,
    pub lookup_key: String,
    pub secret_material: String,
    pub status: PrincipalStatus,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

impl From<CreatePrincipalCredential> for PrincipalCredentialEntry {
    fn from(c: CreatePrincipalCredential) -> Self {
        Self {
            credential_id: c.credential_id,
            principal_id: c.principal_id,
            kind: c.kind,
            lookup_key: c.lookup_key,
            secret_material: c.secret_material,
            status: c.status,
            expires_at: c.expires_at,
            last_used_at: c.last_used_at,
            created_at: c.created_at,
            revoked_at: c.revoked_at,
        }
    }
}

impl From<PrincipalCredentialEntry> for CreatePrincipalCredentialResponse {
    fn from(e: PrincipalCredentialEntry) -> Self {
        Self {
            credential_id: e.credential_id,
            principal_id: e.principal_id,
            kind: e.kind,
            lookup_key: e.lookup_key,
            status: e.status,
            expires_at: e.expires_at,
            last_used_at: e.last_used_at,
            created_at: e.created_at,
            revoked_at: e.revoked_at,
        }
    }
}

impl Action for CreatePrincipalCredential {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::PrincipalCredentialCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let principal_credential_entry = self.into();
        let credential_id = tx
            .principal_credential()
            .insert(&principal_credential_entry)
            .await?;
        let principal_credential_response = principal_credential_entry.into();
        let after = serde_json::json!(principal_credential_response);
        let app_response = AppResponse::CreatePrincipalCredential(principal_credential_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::PrincipalCredential(credential_id)),
        })
    }
}
