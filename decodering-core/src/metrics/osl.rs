use std::sync::Arc;
use std::time::Instant;

use crate::metrics::{Metric, Metrics, Outcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OslOperation {
    Capabilities,
    Describe,
    ListApps,
    ListBackends,
    Get,
    Put,
    Destroy,
    Delete,
    Taint,
    Restore,
    IsTaint,
    Untaint,
    List,
}

impl OslOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::Describe => "describe",
            Self::ListApps => "list_apps",
            Self::ListBackends => "list_backends",
            Self::Get => "get_secret",
            Self::Put => "put_secret",
            Self::Destroy => "destroy_secret",
            Self::Delete => "delete_secret",
            Self::Taint => "taint_secret",
            Self::Restore => "restore_secret",
            Self::IsTaint => "is_tainted_secret",
            Self::Untaint => "untaint_secret",
            Self::List => "list_secret",
        }
    }
}

pub struct OslOp {
    metrics: Arc<dyn Metrics>,
    operation: OslOperation,
    start: Instant,
    outcome: Outcome,
}

impl OslOp {
    pub fn start(metrics: Arc<dyn Metrics>, operation: OslOperation) -> Self {
        Self {
            metrics,
            operation,
            start: Instant::now(),
            outcome: Outcome::Error,
        }
    }
    pub fn ok(&mut self) {
        self.outcome = Outcome::Ok;
    }

    pub fn denied(&mut self) {
        self.outcome = Outcome::Denied;
    }
}

impl Drop for OslOp {
    fn drop(&mut self) {
        self.metrics.record(Metric::OslOperation {
            op: self.operation.as_str(),
            outcome: self.outcome,
            elapsed: self.start.elapsed(),
        });
    }
}
