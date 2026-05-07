// use std::collections::BTreeMap;
// use std::collections::BTreeSet;
// use std::io::Cursor;
// use std::sync::OnceLock;

// use decodering_db::sqlite::SqliteDatabase;
// use openraft::BasicNode;
// use openraft::error::Infallible;
// use openraft::error::decompose::DecomposeResult;
// use openraft::rt::WatchReceiver;
// use zeroize::Zeroizing;

// use crate::NodeId;
// use crate::Raft;
// use crate::StateMachineStore;
// use crate::raft_types::*;

// pub struct App {
//     pub id: NodeId,
//     pub master_key: OnceLock<Zeroizing<Vec<u8>>>,
//     pub addr: String,
//     pub raft: Raft,
//     pub state_machine: StateMachineStore,
//     pub db: SqliteDatabase,
// }

// // Management endpoints
// impl App {
//     pub async fn init(&self, data: Vec<(NodeId, String)>) -> Result<(), InitializeError> {
//         let mut nodes = BTreeMap::new();
//         if data.is_empty() {
//             nodes.insert(
//                 self.id,
//                 BasicNode {
//                     addr: self.addr.clone(),
//                 },
//             );
//         } else {
//             for (id, addr) in data.into_iter() {
//                 nodes.insert(id, BasicNode { addr });
//             }
//         };
//         let res = self.raft.initialize(nodes).await.decompose().unwrap();
//         res
//     }

//     pub async fn metrics(&self) -> Result<RaftMetrics, Infallible> {
//         let metrics = self.raft.metrics().borrow_watched().clone();

//         let res: Result<RaftMetrics, Infallible> = Ok(metrics);
//         res
//     }

//     pub async fn add_learner(
//         &self,
//         req: (NodeId, String),
//     ) -> Result<ClientWriteResponse, ClientWriteError> {
//         let (node_id, api_addr) = req;
//         let node = Node { addr: api_addr };
//         let res = self
//             .raft
//             .add_learner(node_id, node, true)
//             .await
//             .decompose()
//             .unwrap();
//         res
//     }

//     pub async fn change_membership(
//         &self,
//         req: BTreeSet<NodeId>,
//     ) -> Result<ClientWriteResponse, ClientWriteError> {
//         let body = req;
//         let res = self
//             .raft
//             .change_membership(body, false)
//             .await
//             .decompose()
//             .unwrap();
//         res
//     }
// }

// // Raft endpoints
// impl App {
//     pub async fn vote(&self, req: VoteRequest) -> Result<VoteResponse, Infallible> {
//         match self.raft.vote(req).await.decompose() {
//             Ok(infallible_result) => infallible_result,
//             Err(fatal_err) => {
//                 tracing::error!("Raft node is dead: {:?}", fatal_err);
//                 std::process::exit(1);
//             }
//         }
//     }

//     pub async fn append(
//         &self,
//         req: AppendEntriesRequest,
//     ) -> Result<AppendEntriesResponse, Infallible> {
//         match self.raft.append_entries(req).await.decompose() {
//             Ok(infallible_result) => infallible_result,
//             Err(fatal_err) => {
//                 tracing::error!("Raft node is dead: {:?}", fatal_err);
//                 std::process::exit(1);
//             }
//         }
//     }

//     pub async fn snapshot(
//         &self,
//         req: (VoteOf, SnapshotMetaOf, Vec<u8>),
//     ) -> Result<SnapshotResponse, Fatal> {
//         let (vote, meta, data) = req;
//         let snapshot = Snapshot {
//             meta,
//             snapshot: Cursor::new(data),
//         };
//         let result = self.raft.install_full_snapshot(vote, snapshot).await;
//         result
//     }
// }

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use openraft::error::Infallible;
use openraft::error::decompose::DecomposeResult;
use openraft::rt::WatchReceiver;
use openraft::{BasicNode, Snapshot};

use crate::NodeId;
use crate::raft_types::AppendEntriesRequest;
use crate::raft_types::AppendEntriesResponse;
use crate::raft_types::ClientWriteError;
use crate::raft_types::ClientWriteResponse;
use crate::raft_types::Fatal;
use crate::raft_types::InitializeError;
use crate::raft_types::Node;
use crate::raft_types::RaftMetrics;
use crate::raft_types::SnapshotMetaOf;
use crate::raft_types::SnapshotResponse;
use crate::raft_types::VoteOf;
use crate::raft_types::VoteRequest;
use crate::raft_types::VoteResponse;
use crate::{Raft, StateMachineStore};
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut nodes = BTreeMap::new();
        if data.is_empty() {
            nodes.insert(self.id, BasicNode { addr: addr.clone() });
        } else {
            for (id, addr) in data {
                nodes.insert(id, BasicNode { addr });
            }
        }
        self.raft.initialize(nodes).await.decompose()??;
        Ok(())
    }

    pub async fn metrics(&self) -> RaftMetrics {
        self.raft.metrics().borrow_watched().clone()
    }

    pub async fn add_learner(
        &self,
        req: (NodeId, String),
    ) -> Result<ClientWriteResponse, ClientWriteError> {
        let (node_id, api_addr) = req;
        let node = Node { addr: api_addr };
        self.raft
            .add_learner(node_id, node, true)
            .await
            .decompose()
            .unwrap()
    }

    pub async fn change_membership(
        &self,
        req: BTreeSet<NodeId>,
    ) -> Result<ClientWriteResponse, ClientWriteError> {
        self.raft
            .change_membership(req, false)
            .await
            .decompose()
            .unwrap()
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
