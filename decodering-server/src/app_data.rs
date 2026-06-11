use std::sync::Arc;
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

#[derive(Clone)]
pub struct AppData<D: Database> {
    pub master_key: Arc<OnceLock<Zeroizing<Vec<u8>>>>,
    pub addr: String,
    pub db: D,
    pub raft: Option<RaftBits>,
}

impl AppData<SqliteDatabase> {
    pub async fn new_raft<P: AsRef<Path> + Send + Sync>(
        node_id: NodeId,
        dir: P,
        addr: &str,
        auto_migrate: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let components = setup_raft_node(node_id, dir, auto_migrate).await?;
        Ok(Self {
            master_key: Arc::new(OnceLock::new()),
            addr: addr.to_owned(),
            db: components.db,
            raft: Some(RaftBits {
                id: node_id,
                raft: components.raft,
                state_machine: components.state_machine,
            }),
        })
    }

    pub async fn new(url: &str, addr: String, auto_migrate: bool) -> std::io::Result<Self> {
        let db = SqliteDatabase::connect(url, auto_migrate)
            .await
            .map_err(std::io::Error::other)?;

        Ok(Self {
            master_key: Arc::new(OnceLock::new()),
            addr,
            db,
            raft: None,
        })
    }
}

impl AppData<PostgresDatabase> {
    pub async fn new(url: &str, addr: String, auto_migrate: bool) -> std::io::Result<Self> {
        let db = PostgresDatabase::connect(url, auto_migrate)
            .await
            .map_err(std::io::Error::other)?;
        Ok(Self {
            master_key: Arc::new(OnceLock::new()),
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
