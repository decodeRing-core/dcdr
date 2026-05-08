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

impl From<CreateShamirConfiguration> for ShamirEntry {
    fn from(c: CreateShamirConfiguration) -> Self {
        Self {
            total_shares: c.total_shares,
            threshold: c.threshold,
            validation_hash: c.validation_hash,
            created_at: c.timestamp,
        }
    }
}

impl From<ShamirEntry> for CreateShamirConfigurationResponse {
    fn from(e: ShamirEntry) -> Self {
        Self {
            total_shares: e.total_shares,
            threshold: e.threshold,
            timestamp: e.created_at,
        }
    }
}

impl CreateShamirConfiguration {
    pub fn new(total_shares: i16, threshold: i16, validation_hash: Vec<u8>) -> Self {
        Self {
            total_shares,
            threshold,
            validation_hash,
            timestamp: now_ts(),
        }
    }
    pub fn request(total_shares: i16, threshold: i16, validation_hash: Vec<u8>) -> AppRequest {
        let shamir_config = Self::new(total_shares, threshold, validation_hash);
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
        let entry = self.into();
        let id = tx.shamir().insert(&entry).await?;
        let response = entry.into();
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
