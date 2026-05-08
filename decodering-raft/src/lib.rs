#![deny(unused_qualifications)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::sync::Arc;

use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_db::sqlite::SqliteDatabase;
use openraft::{Config, SnapshotPolicy};

use crate::network::network_impl::NetworkFactory;
use crate::rocksdb_log_store::RocksLogStore;
use crate::store::state_machine::new_storage;

pub mod app;
pub mod network;
pub mod raft_types;
pub mod rocksdb_log_store;
pub mod store;

pub type NodeId = u64;

openraft::declare_raft_types!(
    pub TypeConfig:
        D = AppRequest,
        R = AppResponse,
        LeaderId = openraft::impls::leader_id_std::LeaderId<u64, NodeId>,
);

pub type LogStore = RocksLogStore<TypeConfig>;
pub type StateMachineStore = store::state_machine::StateMachineStore<SqliteDatabase>;
pub type Raft = openraft::Raft<TypeConfig, StateMachineStore>;

pub struct RaftComponents {
    pub raft: Raft,
    pub state_machine: StateMachineStore,
    pub db: SqliteDatabase,
}

pub async fn setup_raft_node<P>(
    node_id: NodeId,
    dir: P,
) -> Result<RaftComponents, Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
{
    let config = Config {
        heartbeat_interval: 250,
        election_timeout_min: 299,
        snapshot_policy: SnapshotPolicy::LogsSinceLast(5),
        max_in_snapshot_log_to_keep: 2,
        ..Default::default()
    };

    let validated_config = config.validate()?;
    //let Ok(validated_config) = validated_config?;
    let config = Arc::new(validated_config);

    let (log_store, state_machine_store, sqlite_db) = new_storage(&dir).await?;

    let network = NetworkFactory::new();
    let raft = openraft::Raft::new(
        node_id,
        config.clone(),
        network,
        log_store,
        state_machine_store.clone(),
    )
    .await?;

    Ok(RaftComponents {
        raft,
        state_machine: state_machine_store,
        db: sqlite_db,
    })
}
