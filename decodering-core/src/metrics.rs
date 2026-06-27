use std::time::Duration;

pub mod app_auth_attempt;
pub mod auth_attempt;
pub mod osl;
pub mod plugin_invocation;
pub mod unlock_attempt;

pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";

#[derive(Clone, Copy)]
pub enum Outcome {
    Ok,
    Error,
    Denied,
}

pub enum RaftRole {
    Leader,
    Follower,
    Candidate,
    Learner,
    Shutdown,
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
    AppAuthAttempt {
        method: &'static str,
        outcome: Outcome,
        elapsed: Duration,
    },
    Unlock {
        outcome: Outcome,
    },
    Locked(bool),
    RaftLeader(bool),
    RaftInitialized(bool),
    RaftTerm(u64),
    RaftLearners(usize),
    RaftVoters(usize),
    RaftState(RaftRole),
    RaftHasLeader(bool),
    RaftLastLogIndex(u64),
    RaftCommittedIndex(u64),
    RaftAppliedIndex(u64),
    RaftSnapshotIndex(u64),
    RaftMillisSinceQuorumAck(u64),
    RaftReplicationMatched {
        target: u64,
        index: u64,
    },
    DbPool {
        active: u32,
        idle: u32,
        max: u32,
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
