use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use openraft::error::Infallible;
use openraft::error::decompose::DecomposeResult;
use openraft::rt::WatchReceiver;
use openraft::{BasicNode, Snapshot};

use crate::NodeId;
use crate::raft_types::ClientWriteError;
use crate::raft_types::ClientWriteResponse;
use crate::raft_types::Fatal;
use crate::raft_types::Node;
use crate::raft_types::RaftMetrics;
use crate::raft_types::SnapshotMetaOf;
use crate::raft_types::SnapshotResponse;
use crate::raft_types::VoteOf;
use crate::raft_types::VoteRequest;
use crate::raft_types::VoteResponse;
use crate::raft_types::{AppendEntriesRequest, RaftError};
use crate::raft_types::{AppendEntriesResponse, InitializeError};
use crate::{Raft, StateMachineStore};

#[derive(Clone)]
pub struct RaftBits {
    pub id: NodeId,
    pub raft: Raft,
    pub state_machine: StateMachineStore,
}

impl RaftBits {
    pub async fn init(
        &self,
        addr: String,
        data: Vec<(NodeId, String)>,
    ) -> Result<(), RaftError<InitializeError>> {
        let mut nodes = BTreeMap::new();
        if data.is_empty() {
            nodes.insert(self.id, BasicNode { addr: addr.clone() });
        } else {
            for (id, addr) in data {
                nodes.insert(id, BasicNode { addr });
            }
        }
        self.raft.initialize(nodes).await
    }

    pub fn metrics(&self) -> RaftMetrics {
        self.raft.metrics().borrow_watched().clone()
    }

    pub async fn add_learner(
        &self,
        req: (NodeId, String),
    ) -> Result<ClientWriteResponse, RaftError<ClientWriteError>> {
        let (node_id, api_addr) = req;
        let node = Node { addr: api_addr };
        self.raft.add_learner(node_id, node, true).await
    }

    pub async fn change_membership(
        &self,
        req: BTreeSet<NodeId>,
    ) -> Result<ClientWriteResponse, RaftError<ClientWriteError>> {
        self.raft.change_membership(req, true).await
    }

    pub async fn vote(&self, req: VoteRequest) -> Result<VoteResponse, Infallible> {
        match self.raft.vote(req).await.decompose() {
            Ok(r) => r,
            Err(fatal) => {
                tracing::error!("Raft node is dead: {:?}", fatal);
                std::process::exit(1);
            }
        }
    }

    pub async fn append(
        &self,
        req: AppendEntriesRequest,
    ) -> Result<AppendEntriesResponse, Infallible> {
        match self.raft.append_entries(req).await.decompose() {
            Ok(r) => r,
            Err(fatal) => {
                tracing::error!("Raft node is dead: {:?}", fatal);
                std::process::exit(1);
            }
        }
    }

    pub async fn snapshot(
        &self,
        req: (VoteOf, SnapshotMetaOf, Vec<u8>),
    ) -> Result<SnapshotResponse, Fatal> {
        let (vote, meta, data) = req;
        let snapshot = Snapshot {
            meta,
            snapshot: Cursor::new(data),
        };
        self.raft.install_full_snapshot(vote, snapshot).await
    }
}
