use std::sync::Arc;
use std::time::Instant;

use crate::metrics::{Metric, Metrics, Outcome};

pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";

pub mod op {
    pub const CAPABILITIES: &str = "capabilities";
    pub const DESCRIBE: &str = "describe";
    pub const LIST_APPS: &str = "list_apps";
    pub const LIST_BACKENDS: &str = "list_backends";
    pub const GET: &str = "get_secret";
    pub const PUT: &str = "put_secret";
    pub const DESTROY: &str = "destroy_secret";
    pub const DELETE: &str = "delete_secret";
    pub const TAINT: &str = "taint_secret";
    pub const RESTORE: &str = "restore_secret";
    pub const IS_TAINT: &str = "is_tainted_secret";
    pub const UNTAINT: &str = "untaint_secret";
    pub const LIST: &str = "list_secret";
}

pub struct OslOp {
    metrics: Arc<dyn Metrics>,
    operation: &'static str,
    start: Instant,
    outcome: Outcome,
}

impl OslOp {
    pub fn start(metrics: Arc<dyn Metrics>, operation: &'static str) -> Self {
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
}

impl Drop for OslOp {
    fn drop(&mut self) {
        self.metrics.record(Metric::OslOperation {
            op: self.operation,
            outcome: self.outcome,
            elapsed: self.start.elapsed(),
        });
    }
}
