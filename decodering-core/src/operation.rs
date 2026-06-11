use std::sync::Arc;
use std::time::Instant;

use crate::metrics::{Metric, Metrics, Outcome};

pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAttemptMethod {
    BearerToken,
}

impl AuthAttemptMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BearerToken => "bearer_token",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAuthAttemptMethod {
    None,
    Tpm,
    ApiKey,
    AwsIam,
}

impl AppAuthAttemptMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Tpm => "trusted_platform_module",
            Self::ApiKey => "api_key",
            Self::AwsIam => "aws_iam",
        }
    }
}

impl From<String> for AppAuthAttemptMethod {
    fn from(e: String) -> Self {
        match e.as_str() {
            "trustedPlatformModule" => Self::Tpm,
            "apiKey" => Self::ApiKey,
            "awsIdentity" => Self::AwsIam,
            _ => Self::None, // Fallback for unknown strings
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

pub struct AuthAttempt {
    metrics: Arc<dyn Metrics>,
    method: AuthAttemptMethod,
    start: Instant,
    outcome: Outcome,
}

impl AuthAttempt {
    pub fn start(metrics: Arc<dyn Metrics>, method: AuthAttemptMethod) -> Self {
        Self {
            metrics,
            method,
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

impl Drop for AuthAttempt {
    fn drop(&mut self) {
        self.metrics.record(Metric::AuthAttempt {
            method: self.method.as_str(),
            outcome: self.outcome,
            elapsed: self.start.elapsed(),
        });
    }
}

pub struct AppAuthAttempt {
    metrics: Arc<dyn Metrics>,
    method: AppAuthAttemptMethod,
    start: Instant,
    outcome: Outcome,
}

impl AppAuthAttempt {
    pub fn start(metrics: Arc<dyn Metrics>, method: AppAuthAttemptMethod) -> Self {
        Self {
            metrics,
            method,
            start: Instant::now(),
            outcome: Outcome::Error,
        }
    }

    pub fn method(&mut self, method: AppAuthAttemptMethod) {
        self.method = method;
    }

    pub fn ok(&mut self) {
        self.outcome = Outcome::Ok;
    }

    pub fn denied(&mut self) {
        self.outcome = Outcome::Denied;
    }
}

impl Drop for AppAuthAttempt {
    fn drop(&mut self) {
        self.metrics.record(Metric::AppAuthAttempt {
            method: self.method.as_str(),
            outcome: self.outcome,
            elapsed: self.start.elapsed(),
        });
    }
}
