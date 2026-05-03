use decodering_db::{DbError, Tx};

use crate::audit::{AuditCapture, AuditDescriptor};
use crate::error::{DenyReason, ExecutionError};

pub trait Action: Send + Sync {
    type Output: Send + AuditCapture;

    fn audit_descriptor(&self) -> AuditDescriptor;

    fn policy_check(&self) -> impl Future<Output = Result<Option<DenyReason>, DbError>> + Send {
        async { Ok(None) }
    }

    fn execute<U: Tx + Send>(
        self,
        tx: &mut U,
    ) -> impl Future<Output = Result<Self::Output, ExecutionError>> + Send;
}
