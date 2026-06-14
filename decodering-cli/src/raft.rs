#![allow(clippy::print_stdout)]
use std::error::Error;

use clap::{Args, Subcommand};

use crate::output;
use crate::source::ValueSource;
use crate::{api, prompt};

#[derive(Subcommand)]
pub enum RaftCommand {
    /// Initialize the raft cluster
    Init(RaftInitInput),
    /// Shut down the raft node
    Shutdown,
    /// Add a learner node to the cluster
    AddLearner(AddLearnerInput),
}

#[derive(Args)]
pub struct RaftInitInput {
    /// JSON params (inline or `@file`). When omitted, you'll be prompted for nodes.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct AddLearnerInput {
    /// JSON params `[<node_id>, "<addr>"]` (inline or `@file`). When omitted, you'll be prompted.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

pub async fn run(cmd: RaftCommand, addr: &str) -> Result<(), Box<dyn Error>> {
    match cmd {
        RaftCommand::Init(input) => init(input, addr).await,
        RaftCommand::Shutdown => shutdown(addr).await,
        RaftCommand::AddLearner(input) => add_learner(input, addr).await,
    }
}

async fn init(input: RaftInitInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let req: api::RaftInitRequest = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => api::RaftInitRequest { raft_init: vec![] },
    };
    let resp = api::raft_init(addr, req).await?;
    output::report(&resp);
    Ok(())
}

async fn shutdown(addr: &str) -> Result<(), Box<dyn Error>> {
    let resp = api::raft_shutdown(addr).await?;
    output::report(&resp);
    Ok(())
}

async fn add_learner(input: AddLearnerInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let node: (u64, String) = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => prompt_node()?,
    };
    let resp = api::raft_add_learner(addr, node).await?;
    output::report(&resp);
    Ok(())
}

fn prompt_node() -> Result<(u64, String), Box<dyn Error>> {
    let id: u64 = prompt::line("Node id: ")?
        .parse()
        .map_err(|_| "node id must be a number")?;
    let address = prompt::line("Address: ")?;
    if address.is_empty() {
        return Err("address is required".into());
    }
    Ok((id, address))
}
