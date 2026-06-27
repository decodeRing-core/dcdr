//! Declare the Raft type with the `TypeConfig`.

pub use super::TypeConfig;

pub type Vote = <TypeConfig as openraft::RaftTypeConfig>::Vote;
pub type LogId = openraft::alias::LogIdOf<TypeConfig>;
pub type StoredMembership = openraft::alias::StoredMembershipOf<TypeConfig>;

pub type Node = <TypeConfig as openraft::RaftTypeConfig>::Node;

pub type EntryPayload = openraft::alias::EntryPayloadOf<TypeConfig>;
pub type VoteOf = openraft::type_config::alias::VoteOf<TypeConfig>;
pub type SnapshotMetaOf = openraft::type_config::alias::SnapshotMetaOf<TypeConfig>;

pub type SnapshotMeta = openraft::alias::SnapshotMetaOf<TypeConfig>;
pub type Snapshot = openraft::alias::SnapshotOf<TypeConfig>;
pub type SnapshotData = <TypeConfig as openraft::RaftTypeConfig>::SnapshotData;

pub type Fatal = openraft::errors::Fatal<TypeConfig>;
pub type RaftError<E = openraft::errors::Infallible> = openraft::errors::RaftError<TypeConfig, E>;
pub type RPCError<E = openraft::errors::Infallible> = openraft::errors::RPCError<TypeConfig, E>;

pub type StreamingError = openraft::errors::StreamingError<TypeConfig>;

pub type RaftMetrics = openraft::RaftMetrics<TypeConfig>;

pub type ClientWriteError = openraft::errors::ClientWriteError<TypeConfig>;
pub type InitializeError = openraft::errors::InitializeError<TypeConfig>;

pub type VoteRequest = openraft::raft::VoteRequest<TypeConfig>;
pub type VoteResponse = openraft::raft::VoteResponse<TypeConfig>;
pub type AppendEntriesRequest = openraft::raft::AppendEntriesRequest<TypeConfig>;
pub type AppendEntriesResponse = openraft::raft::AppendEntriesResponse<TypeConfig>;
pub type InstallSnapshotRequest = openraft::raft::InstallSnapshotRequest<TypeConfig>;
pub type SnapshotResponse = openraft::raft::SnapshotResponse<TypeConfig>;
pub type ClientWriteResponse = openraft::raft::ClientWriteResponse<TypeConfig>;
pub type ServerState = openraft::ServerState;
pub type ReadPolicy = openraft::ReadPolicy;
pub type TokioInstant = openraft::TokioInstant;
pub use openraft::Instant;
