use decodering_raft::NodeId;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct InitRaftRequestData {
    pub raft_init: Vec<(NodeId, String)>,
}
