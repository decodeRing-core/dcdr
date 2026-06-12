use std::sync::Arc;
use std::time::Instant;

use crate::metrics::{Metric, Metrics, Outcome};

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
