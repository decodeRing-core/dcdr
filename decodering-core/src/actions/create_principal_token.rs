use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::{PrincipalTokenEntry, PrincipalTokenRepository};
use crate::response::{AppResponse, CreatePrincipalTokenResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreatePrincipalToken {
    pub token_id: String,
    pub token_hash: String,
    pub principal_id: String,
    pub credential_id: Option<String>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
}

impl Action for CreatePrincipalToken {
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
        let principal_token_entry = PrincipalTokenEntry {
            token_id: self.token_id,
            token_hash: self.token_hash,
            principal_id: self.principal_id,
            credential_id: self.credential_id,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            revoked_at: self.revoked_at,
        };
        let token_id = tx.principal_token().insert(&principal_token_entry).await?;
        let principal_token_response = CreatePrincipalTokenResponse {
            token_id: token_id.clone(),
            token_hash: principal_token_entry.token_hash,
            principal_id: principal_token_entry.principal_id,
            credential_id: principal_token_entry.credential_id,
            issued_at: principal_token_entry.issued_at,
            expires_at: principal_token_entry.expires_at,
            revoked_at: principal_token_entry.revoked_at,
        };
        let after = serde_json::json!(principal_token_response);
        let app_response = AppResponse::CreatePrincipalToken(principal_token_response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::PrincipalToken(token_id)),
        })
    }
}
