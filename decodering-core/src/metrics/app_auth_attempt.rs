use std::sync::Arc;
use std::time::Instant;

use crate::metrics::{Metric, Metrics, Outcome};

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
            _ => Self::None,
        }
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
