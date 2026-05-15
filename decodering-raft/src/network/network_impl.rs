use std::future::Future;

use openraft::BasicNode;
use openraft::OptionalSend;
use openraft::RaftNetworkFactory;
use openraft::errors::ReplicationClosed;
use openraft::network::RPCOption;
use openraft::network::v2::RaftNetworkV2;
use reqwest::Client;

use crate::NodeId;
use crate::TypeConfig;
use crate::raft_types::AppendEntriesRequest;
use crate::raft_types::AppendEntriesResponse;
use crate::raft_types::RPCError;
use crate::raft_types::RaftError;
use crate::raft_types::Snapshot;
use crate::raft_types::SnapshotResponse;
use crate::raft_types::StreamingError;
use crate::raft_types::Vote;
use crate::raft_types::VoteRequest;
use crate::raft_types::VoteResponse;

#[derive(Clone)]
pub struct NetworkFactory {
    client: Client,
}

impl Default for NetworkFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkFactory {
    pub fn new() -> Self {
        #[allow(clippy::expect_used)]
        Self {
            client: Client::builder()
                .no_proxy()
                .build()
                .expect("Failed to build reqwest client"),
        }
    }
}

impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = Connection;

    #[tracing::instrument(level = "debug", skip_all)]
    async fn new_client(&mut self, target: NodeId, node: &BasicNode) -> Self::Network {
        Connection {
            addr: node.addr.clone(),
            client: self.client.clone(),
            _target: target,
        }
    }
}

pub struct Connection {
    addr: String,
    client: Client,
    _target: NodeId,
}

impl Connection {
    async fn request<Req, Resp>(&self, path: &str, req: Req) -> Result<Resp, RPCError>
    where
        Req: serde::Serialize,
        Result<Resp, RaftError>: serde::de::DeserializeOwned,
    {
        let url = format!("http://{}/{}", self.addr, path);
        tracing::trace!(">>> network send request to {}: {}", url, path);

        let resp = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() || e.is_timeout() {
                    openraft::errors::RPCError::Unreachable(openraft::errors::Unreachable::new(&e))
                } else {
                    openraft::errors::RPCError::Network(openraft::errors::NetworkError::new(&e))
                }
            })?;

        let status = resp.status();
        let body_bytes = resp.bytes().await.map_err(|e| {
            openraft::errors::RPCError::Network(openraft::errors::NetworkError::new(&e))
        })?;
        let body_str = String::from_utf8_lossy(&body_bytes);

        if !status.is_success() {
            tracing::warn!("Node {} returned HTTP {}: {}", url, status, body_str);
        }

        let res: Result<Resp, openraft::errors::RaftError<TypeConfig>> =
            serde_json::from_str(&body_str).map_err(|e| {
                tracing::error!(
                    "Failed to parse JSON from Node {}. Raw body: {}",
                    url,
                    body_str
                );
                openraft::errors::RPCError::Network(openraft::errors::NetworkError::new(&e))
            })?;
        res.map_err(|e| {
            openraft::errors::RPCError::Unreachable(openraft::errors::Unreachable::new(&e))
        })
    }
}

impl RaftNetworkV2<TypeConfig> for Connection {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse, RPCError> {
        let resp = self.request("append", req).await?;
        Ok(resp)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn vote(
        &mut self,
        req: VoteRequest,
        _option: RPCOption,
    ) -> Result<VoteResponse, RPCError> {
        let resp = self.request("vote", req).await?;
        Ok(resp)
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn full_snapshot(
        &mut self,
        vote: Vote,
        snapshot: Snapshot,
        _cancel: impl Future<Output = ReplicationClosed> + OptionalSend + 'static,
        _option: RPCOption,
    ) -> Result<SnapshotResponse, StreamingError> {
        let data: Vec<u8> = snapshot.snapshot.into_inner();
        let req = (vote, snapshot.meta, data);

        let resp = self.request("snapshot", req).await.map_err(|e| match e {
            openraft::errors::RPCError::Timeout(t) => openraft::errors::StreamingError::Timeout(t),
            openraft::errors::RPCError::Unreachable(u) => {
                openraft::errors::StreamingError::Unreachable(u)
            }
            openraft::errors::RPCError::Network(n) => openraft::errors::StreamingError::Network(n),
            openraft::errors::RPCError::RemoteError(r) => {
                openraft::errors::StreamingError::Network(openraft::errors::NetworkError::new(&r))
            }
        })?;
        Ok(resp)
    }
}
