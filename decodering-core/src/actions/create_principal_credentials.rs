use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::domain::{PrincipalKind, PrincipalStatus};
use crate::error::ExecutionError;
use crate::repository::{PrincipalCredentialEntry, PrincipalCredentialRepository};
use crate::response::{AppResponse, CreatePrincipalCredentialResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalCredential {
    pub credential_id: String,
    pub principal_id: String,
    pub kind: PrincipalKind,
    pub lookup_key: String,
    pub secret_material: String,
    pub status: PrincipalStatus,
    pub expires_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub revoked_at: Option<i64>,
}

impl Action for CreatePrincipalCredential {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::PrincipalCredentialCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let principal_credential_entry = PrincipalCredentialEntry {
            credential_id: self.credential_id,
            principal_id: self.principal_id,
            kind: self.kind,
            lookup_key: self.lookup_key,
            secret_material: self.secret_material,
            status: self.status,
            expires_at: self.expires_at,
            last_used_at: self.last_used_at,
            created_at: self.created_at,
            revoked_at: self.revoked_at,
        };
        let credential_id = tx
            .principal_credential()
            .insert(&principal_credential_entry)
            .await?;
        let principal_credential_response = CreatePrincipalCredentialResponse {
            credential_id: credential_id.clone(),
            principal_id: principal_credential_entry.principal_id,
            kind: principal_credential_entry.kind,
            lookup_key: principal_credential_entry.lookup_key,
            secret_material: principal_credential_entry.secret_material,
            status: principal_credential_entry.status,
            expires_at: principal_credential_entry.expires_at,
            last_used_at: principal_credential_entry.last_used_at,
            created_at: principal_credential_entry.created_at,
            revoked_at: principal_credential_entry.revoked_at,
        };
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
