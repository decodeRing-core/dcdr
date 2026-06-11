use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Outcome {
    Ok,
    Error,
}

pub enum Metric<'a> {
    OslOperation {
        op: &'static str,
        outcome: Outcome,
        elapsed: Duration,
    },
    AuthAttempt {
        method: &'static str,
        outcome: Outcome,
        elapsed: Duration,
    },
    TokenValidation {
        outcome: Outcome,
    },
    Unseal {
        outcome: Outcome,
    },
    Sealed(bool),
    RaftLeader(bool),
    RaftInitialized(bool),
    RaftTerm(u64),
    RaftLeaderChange,
    RaftPeers(u32),
    StorageQuery {
        backend: &'static str,
        op: &'static str,
        outcome: Outcome,
        elapsed: Duration,
    },
    PluginInvocation {
        plugin: &'a str,
        outcome: Outcome,
        elapsed: Duration,
    },
    PluginsLoaded(u32),
}

pub trait Metrics: Send + Sync {
    fn record(&self, metric: Metric<'_>);
}

pub struct NoopMetrics;
impl Metrics for NoopMetrics {
    fn record(&self, _: Metric<'_>) {}
}
