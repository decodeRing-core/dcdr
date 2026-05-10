use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::actions::create_principal::CreatePrincipal;
use crate::actions::create_principal_credential::CreatePrincipalCredential;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::repository::PrincipalCredentialEntry;
use crate::repository::PrincipalCredentialRepository;
use crate::repository::PrincipalRepository;
use crate::request::AppRequest;
use crate::response::{AppResponse, CreateAppUserResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateAppUser {
    pub user_id: i64,
    pub principal: CreatePrincipal,
    pub principal_credential: CreatePrincipalCredential,
}

impl CreateAppUser {
    pub fn request(
        user_id: i64,
        principal: CreatePrincipal,
        principal_credential: CreatePrincipalCredential,
    ) -> AppRequest {
        let app_user = Self {
            user_id,
            principal,
            principal_credential,
        };
        AppRequest::CreateAppUser(app_user)
    }
}

impl Action for CreateAppUser {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::AppUserCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let principal_entry = self.principal.into();
        let principal_id = tx.principal().insert(&principal_entry).await?;
        let principal_response = principal_entry.into();

        let mut principal_credential_entry: PrincipalCredentialEntry =
            self.principal_credential.into();
        principal_credential_entry.principal_id = principal_id;
        let _ = tx
            .principal_credential()
            .insert(&principal_credential_entry)
            .await?;

        let principal_credential_response = principal_credential_entry.into();

        let response = CreateAppUserResponse {
            principal: principal_response,
            principal_credential: principal_credential_response,
        };
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreateAppUser(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::User(self.user_id)),
        })
    }
}
