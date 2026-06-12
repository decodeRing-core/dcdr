use std::sync::Arc;
use std::time::Instant;

use crate::metrics::{Metric, Metrics, Outcome};

pub struct PluginInvocation {
    metrics: Arc<dyn Metrics>,
    plugin: String,
    start: Instant,
    outcome: Outcome,
}

impl PluginInvocation {
    pub fn start(metrics: Arc<dyn Metrics>, plugin: String) -> Self {
        Self {
            metrics,
            plugin,
            start: Instant::now(),
            outcome: Outcome::Error,
        }
    }
    pub fn ok(&mut self) {
        self.outcome = Outcome::Ok;
    }
}

impl Drop for PluginInvocation {
    fn drop(&mut self) {
        self.metrics.record(Metric::PluginInvocation {
            outcome: self.outcome,
            elapsed: self.start.elapsed(),
            plugin: &self.plugin,
        });
    }
}
