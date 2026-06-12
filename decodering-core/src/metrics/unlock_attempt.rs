use std::sync::Arc;

use crate::metrics::{Metric, Metrics, Outcome};

pub struct UnlockAttempt {
    metrics: Arc<dyn Metrics>,
    outcome: Outcome,
}

impl UnlockAttempt {
    pub fn start(metrics: Arc<dyn Metrics>) -> Self {
        Self {
            metrics,
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

impl Drop for UnlockAttempt {
    fn drop(&mut self) {
        self.metrics.record(Metric::Unlock {
            outcome: self.outcome,
        });
    }
}
