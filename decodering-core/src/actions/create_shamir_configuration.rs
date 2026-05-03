use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, ActionOutput, Actor, AuditDescriptor, Target};
use crate::error::ExecutionError;
use crate::now_ts;
use crate::repository::{ShamirEntry, ShamirRepository};
use crate::request::AppRequest;
use crate::response::{AppResponse, CreateShamirConfigurationResponse};
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct CreateShamirConfiguration {
    pub total_shares: i16,
    pub threshold: i16,
    pub validation_hash: Vec<u8>,
    pub timestamp: i64,
}

impl CreateShamirConfiguration {
    pub fn new(
        total_shares: i16,
        threshold: i16,
        validation_hash: Vec<u8>,
    ) -> CreateShamirConfiguration {
        CreateShamirConfiguration {
            total_shares,
            threshold,
            validation_hash,
            timestamp: now_ts(),
        }
    }
    pub fn request(total_shares: i16, threshold: i16, validation_hash: Vec<u8>) -> AppRequest {
        let shamir_config =
            CreateShamirConfiguration::new(total_shares, threshold, validation_hash);
        AppRequest::CreateShamirConfiguration(shamir_config)
    }
}

impl Action for CreateShamirConfiguration {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: Actor::None,
            action_kind: ActionKind::ShamirConfigurationCreate,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let entry = ShamirEntry {
            total_shares: self.total_shares,
            threshold: self.threshold,
            validation_hash: self.validation_hash,
            created_at: self.timestamp,
        };
        let id = tx.shamir().insert(&entry).await?;
        let response = CreateShamirConfigurationResponse {
            id,
            total_shares: entry.total_shares,
            threshold: entry.threshold,
            validation_hash: entry.validation_hash,
            timestamp: entry.created_at,
        };
        let after = serde_json::json!(response);
        let app_response = AppResponse::CreateShamirConfiguration(response);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: Some(after),
            target: Some(Target::ShamirConfiguration(id)),
        })
    }
}
