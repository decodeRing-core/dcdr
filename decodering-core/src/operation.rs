use metrics::{counter, histogram};
use std::time::Instant;

pub const OSL_OPS: &str = "dcdr_osl_operations_total";
pub const OSL_OP_DURATION: &str = "dcdr_osl_operation_duration_seconds";
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
pub mod outcome {
    pub const OK: &str = "ok";
    pub const ERROR: &str = "error";
}

pub struct OSLOperation {
    operation: &'static str,
    start: Instant,
    outcome: &'static str,
}

impl OSLOperation {
    pub fn start(operation: &'static str) -> Self {
        Self {
            operation,
            start: Instant::now(),
            outcome: outcome::ERROR,
        }
    }
    pub fn ok(&mut self) {
        self.outcome = outcome::OK;
    }
}

impl Drop for OSLOperation {
    fn drop(&mut self) {
        counter!(OSL_OPS, "operation" => self.operation, "status" => self.outcome).increment(1);
        histogram!(OSL_OP_DURATION, "operation" => self.operation)
            .record(self.start.elapsed().as_secs_f64());
    }
}
