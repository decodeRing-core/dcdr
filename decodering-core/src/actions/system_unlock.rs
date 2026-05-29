use blahaj::Share;
use serde::{Deserialize, Serialize};

use crate::action::Action;
use crate::audit::{ActionKind, Target};
use crate::audit::{ActionOutput, Actor, AuditDescriptor};
use crate::error::ExecutionError;
use crate::request::AppRequest;
use crate::response::AppResponse;
use crate::shamir::unlock;
use crate::tx::Tx;

#[derive(Serialize, Debug, Deserialize)]
pub struct SystemUnlock {
    pub actor: Actor,
    pub threshold: u8,
    pub expected_hash: Vec<u8>,
    pub shards: Vec<Vec<u8>>,
}

impl SystemUnlock {
    pub fn request(
        actor: Actor,
        threshold: u8,
        expected_hash: Vec<u8>,
        shards: Vec<Vec<u8>>,
    ) -> AppRequest {
        let app = Self {
            actor,
            threshold,
            expected_hash,
            shards,
        };
        AppRequest::SystemUnlock(app)
    }
}

impl Action for SystemUnlock {
    type Output = ActionOutput<AppResponse>;

    fn audit_descriptor(&self) -> AuditDescriptor {
        AuditDescriptor {
            actor: self.actor.clone(),
            action_kind: ActionKind::SystemUnlock,
            revertible: true,
            undoes: None,
            metadata: None,
        }
    }

    async fn execute<U: Tx + Send>(self, _tx: &mut U) -> Result<Self::Output, ExecutionError> {
        let shares: Option<Vec<Share>> = self
            .shards
            .iter()
            .map(|b| Share::try_from(b.as_slice()))
            .collect::<Result<_, _>>()
            .ok();
        let Some(shares) = shares else {
            tracing::error!("Failed to unlock node. Failed to process shards.");
            return Err(ExecutionError::Action("Invalid shares".to_owned()));
        };
        let secret = unlock(self.threshold, &self.expected_hash, &shares)?;
        let app_response = AppResponse::SystemUnlock(secret);
        Ok(ActionOutput {
            response: app_response,
            before_state: None,
            after_state: None,
            target: Some(Target::System),
        })
    }
}
