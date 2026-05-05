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

impl From<CreatePrincipalToken> for PrincipalTokenEntry {
    fn from(c: CreatePrincipalToken) -> Self {
        Self {
            token_id: c.token_id,
            token_hash: c.token_hash,
            principal_id: c.principal_id,
            credential_id: c.credential_id,
            issued_at: c.issued_at,
            expires_at: c.expires_at,
            revoked_at: c.revoked_at,
        }
    }
}

impl From<PrincipalTokenEntry> for CreatePrincipalTokenResponse {
    fn from(e: PrincipalTokenEntry) -> Self {
        Self {
            token_id: e.token_id,
            principal_id: e.principal_id,
            credential_id: e.credential_id,
            issued_at: e.issued_at,
            expires_at: e.expires_at,
            revoked_at: e.revoked_at,
        }
    }
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
        let principal_token_entry = self.into();
        let token_id = tx.principal_token().insert(&principal_token_entry).await?;
        let principal_token_response = principal_token_entry.into();
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
