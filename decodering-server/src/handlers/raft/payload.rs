use decodering_raft::NodeId;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct InitRaftRequestData {
    pub raft_init: Vec<(NodeId, String)>,
}
