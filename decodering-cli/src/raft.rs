#![allow(clippy::print_stdout)]
use std::collections::{BTreeMap, BTreeSet};
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
    /// Node Metrics
    Metrics,
    /// Change cluster membership
    #[command(subcommand)]
    ChangeMembership(ChangeMembershipCommand),
}

#[derive(Subcommand)]
pub enum ChangeMembershipCommand {
    /// Upgrade existing learners to voters, by id
    AddVoterIds(IdsInput),
    /// Add voters with their node addresses
    AddVoters(NodesInput),
    /// Remove voters
    RemoveVoters(IdsInput),
    /// Replace the entire voter set
    ReplaceAllVoters(IdsInput),
    /// Add nodes as learners
    AddNodes(NodesInput),
    /// Add or replace nodes
    SetNodes(NodesInput),
    /// Remove nodes
    RemoveNodes(IdsInput),
    /// Replace all nodes
    ReplaceAllNodes(NodesInput),
    /// Apply multiple changes in order (JSON only)
    Batch(BatchInput),
}

#[derive(Args)]
pub struct IdsInput {
    /// IDs as JSON array (inline or `@file`), e.g. `[1,2]`. When omitted, you'll be prompted.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct NodesInput {
    /// Nodes as JSON map (inline or `@file`), e.g. `{"1":{"addr":"host:port"}}`.
    /// When omitted, you'll be prompted.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct BatchInput {
    /// Array of changes as JSON (inline or `@file`), e.g.
    /// `[{"AddVoters":{"1":{"addr":"host:port"}}},{"RemoveVoters":[3]}]`
    #[arg(long, value_name = "SOURCE")]
    params: ValueSource,
}

#[derive(Args)]
pub struct ChangeMembershipInput {
    /// Membership change as JSON (inline or `@file`), e.g.
    /// `{"AddVoters":{"1":{"addr":"host:port"}}}`. When omitted, you'll be
    /// prompted to add voters.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
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
        RaftCommand::Metrics => metrics(addr).await,
        RaftCommand::ChangeMembership(cmd) => change_membership(cmd, addr).await,
    }
}

async fn metrics(addr: &str) -> Result<(), Box<dyn Error>> {
    let resp = api::raft_metrics(addr).await?;
    output::report(&resp);
    Ok(())
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

async fn change_membership(cmd: ChangeMembershipCommand, addr: &str) -> Result<(), Box<dyn Error>> {
    use ChangeMembershipCommand as C;
    use api::ChangeMembers;

    let change = match cmd {
        C::AddVoterIds(i) => ChangeMembers::AddVoterIds(ids(&i)?),
        C::AddVoters(i) => ChangeMembers::AddVoters(nodes(&i)?),
        C::RemoveVoters(i) => ChangeMembers::RemoveVoters(ids(&i)?),
        C::ReplaceAllVoters(i) => ChangeMembers::ReplaceAllVoters(ids(&i)?),
        C::AddNodes(i) => ChangeMembers::AddNodes(nodes(&i)?),
        C::SetNodes(i) => ChangeMembers::SetNodes(nodes(&i)?),
        C::RemoveNodes(i) => ChangeMembers::RemoveNodes(ids(&i)?),
        C::ReplaceAllNodes(i) => ChangeMembers::ReplaceAllNodes(nodes(&i)?),
        C::Batch(i) => ChangeMembers::Batch(serde_json::from_str(&i.params.read()?)?),
    };

    let resp = api::raft_change_membership(addr, change).await?;
    output::report(&resp);
    Ok(())
}

fn ids(input: &IdsInput) -> Result<BTreeSet<u64>, Box<dyn Error>> {
    match &input.params {
        Some(src) => Ok(serde_json::from_str(&src.read()?)?),
        None => prompt_ids(),
    }
}

fn nodes(input: &NodesInput) -> Result<BTreeMap<u64, api::Node>, Box<dyn Error>> {
    match &input.params {
        Some(src) => Ok(serde_json::from_str(&src.read()?)?),
        None => prompt_nodes_map(),
    }
}

fn prompt_ids() -> Result<BTreeSet<u64>, Box<dyn Error>> {
    let mut ids = BTreeSet::new();
    loop {
        let line = prompt::line(&format!("Node id {} (empty to finish): ", ids.len() + 1))?;
        if line.is_empty() {
            break;
        }
        ids.insert(line.parse::<u64>().map_err(|_| "id must be a number")?);
    }
    if ids.is_empty() {
        return Err("at least one id is required".into());
    }
    Ok(ids)
}

fn prompt_nodes_map() -> Result<BTreeMap<u64, api::Node>, Box<dyn Error>> {
    let mut nodes = BTreeMap::new();
    loop {
        let id_line = prompt::line(&format!("Node {} id (empty to finish): ", nodes.len() + 1))?;
        if id_line.is_empty() {
            break;
        }
        let id: u64 = id_line.parse().map_err(|_| "id must be a number")?;
        let address = prompt::line("  address: ")?;
        if address.is_empty() {
            return Err("address is required".into());
        }
        nodes.insert(id, api::Node { addr: address });
    }
    if nodes.is_empty() {
        return Err("at least one node is required".into());
    }
    Ok(nodes)
}
