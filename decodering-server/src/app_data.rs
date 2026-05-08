use std::{path::Path, sync::OnceLock};

use decodering_core::request::AppRequest;
use decodering_core::response::AppResponse;
use decodering_core::tx::Database;
use decodering_db::postgres::PostgresDatabase;
use decodering_db::sqlite::SqliteDatabase;
use decodering_raft::NodeId;
use decodering_raft::app::RaftBits;
use decodering_raft::setup_raft_node;
use zeroize::Zeroizing;

use crate::error::AppError;

pub struct AppData<D: Database> {
    pub master_key: OnceLock<Zeroizing<Vec<u8>>>,
    pub addr: String,
    pub db: D,
    pub raft: Option<RaftBits>,
}

impl AppData<SqliteDatabase> {
    pub async fn init_raft<P: AsRef<Path> + Send + Sync>(
        node_id: NodeId,
        dir: P,
        addr: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let components = setup_raft_node(node_id, dir).await?;
        Ok(Self {
            master_key: OnceLock::new(),
            addr,
            db: components.db,
            raft: Some(RaftBits {
                id: node_id,
                raft: components.raft,
                state_machine: components.state_machine,
            }),
        })
    }
}

impl AppData<PostgresDatabase> {
    pub async fn new(url: &str, addr: String) -> std::io::Result<Self> {
        let db = PostgresDatabase::connect(url)
            .await
            .map_err(std::io::Error::other)?;
        Ok(Self {
            master_key: OnceLock::new(),
            addr,
            db,
            raft: None,
        })
    }
}

impl<D: Database> AppData<D> {
    pub async fn submit(&self, req: AppRequest) -> Result<AppResponse, AppError> {
        match &self.raft {
            Some(r) => r
                .raft
                .client_write(req)
                .await
                .map(|r| r.data)
                .map_err(AppError::Raft),
            None => req.run_direct(&self.db).await.map_err(AppError::Action),
        }
    }
}
