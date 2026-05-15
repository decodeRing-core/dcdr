use std::{net::TcpListener, time::Duration};

use actix_web::{
    App, HttpServer,
    rt::{net::TcpStream, spawn, task::JoinHandle, time},
    web::Data,
};
use decodering_db::sqlite::SqliteDatabase;
use decodering_raft::Raft;
use decodering_server::routes::config::config_app;

use crate::common::{init_tracing_once, sqlite_raft_storage, test_config};

pub struct Node {
    pub id: u64,
    pub addr: String,
    pub raft: Raft,
    handle: JoinHandle<std::io::Result<()>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub async fn spawn_node(id: u64) -> Result<Node, Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let addr = listener.local_addr()?.to_string();

    let config = test_config();
    init_tracing_once(&config, &addr);

    let (orchestrator, app_data) = sqlite_raft_storage(&config, id, &addr)
        .await?
        .ok_or("sqlite_raft_storage returned None")?;
    let raft = app_data
        .raft
        .as_ref()
        .ok_or("raft not initialized")?
        .raft
        .clone();

    let config_data = Data::new(config);
    let app_data_wrapper = Data::new(app_data);
    let orchestrator_data = Data::new(orchestrator);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(config_data.clone())
            .app_data(app_data_wrapper.clone())
            .app_data(orchestrator_data.clone())
            .configure(config_app::<SqliteDatabase>)
    })
    .workers(1)
    .listen(listener)?
    .run();

    let handle = spawn(server);

    for _ in 0..100 {
        if TcpStream::connect(&addr).await.is_ok() {
            break;
        }
        time::sleep(Duration::from_millis(20)).await;
    }

    Ok(Node {
        id,
        addr,
        raft,
        handle,
    })
}
