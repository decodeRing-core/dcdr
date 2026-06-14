#![allow(clippy::print_stdout)]
use clap::{Args, Subcommand};
use serde::Deserialize;
use std::error::Error;

use crate::api::{self, PluginsCredentials};
use crate::source::{SecretSource, ValueSource};
use crate::token_store::{self};
use crate::{output, prompt};

#[derive(Subcommand)]
pub enum SystemCommand {
    /// Initialize the system: generates Shamir shards and a root key
    Init(InitInput),
    /// Unlock the system with a threshold of shards
    Unlock(UnlockInput),
    /// Report system status
    Status,
}

#[derive(Args)]
pub struct UnlockInput {
    /// Shards as JSON (inline or `@file`). When omitted, you'll be prompted.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,
}

#[derive(Args)]
pub struct InitInput {
    /// Non-secret params (inline JSON or `@file`). When omitted, you'll be prompted.
    #[arg(long, value_name = "SOURCE")]
    params: Option<ValueSource>,

    /// Optional plugin credentials from a file (`@path`) or stdin (`-`)
    #[arg(long, value_name = "FILE|-")]
    plugins_credentials: Option<SecretSource>,

    /// Initialize the raft cluster before initializing the system
    #[arg(long)]
    with_raft: bool,
}

#[derive(Deserialize)]
struct InitParams {
    total_shares: u8,
    threshold: u8,
}

pub async fn run(cmd: SystemCommand, addr: &str) -> Result<(), Box<dyn Error>> {
    match cmd {
        SystemCommand::Init(input) => init(input, addr).await,
        SystemCommand::Unlock(input) => unlock(input, addr).await,
        SystemCommand::Status => status(addr).await,
    }
}

async fn unlock(input: UnlockInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let shards = match &input.params {
        Some(src) => serde_json::from_str::<api::UnlockRequest>(&src.read()?)?.shards,
        None => prompt_shards()?,
    };
    if shards.is_empty() {
        return Err("at least one shard is required".into());
    }
    let resp = api::unlock(addr, api::UnlockRequest { shards }).await?;
    output::report(&resp);
    Ok(())
}

fn prompt_shards() -> Result<Vec<String>, Box<dyn Error>> {
    let mut shards = Vec::new();
    loop {
        let prompt = format!("Enter shard {} (leave empty to finish): ", shards.len() + 1);
        let shard = rpassword::prompt_password(prompt)?;
        let shard = shard.trim().to_owned();
        if shard.is_empty() {
            break;
        }
        shards.push(shard);
    }
    Ok(shards)
}

async fn status(addr: &str) -> Result<(), Box<dyn Error>> {
    let resp = api::status(addr).await?;
    output::report(&resp);
    Ok(())
}

async fn init(input: InitInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let params: InitParams = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => prompt_init_params()?,
    };
    if params.threshold == 0 || params.threshold > params.total_shares {
        return Err("threshold must be between 1 and total_shares".into());
    }

    let plugins_credentials: PluginsCredentials = match &input.plugins_credentials {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => PluginsCredentials::new(),
    };

    if input.with_raft {
        let resp = api::raft_init(addr, api::RaftInitRequest { raft_init: vec![] }).await?;
        output::report(&resp);
    }

    let res = api::init(
        addr,
        api::InitRequest {
            total_shares: params.total_shares,
            threshold: params.threshold,
            plugins_credentials,
        },
    )
    .await?;

    println!("System initialized.\n");
    println!("Unseal shards — distribute these to separate operators (shown once):\n");
    for (i, shard) in res.shards.iter().enumerate() {
        println!("  Shard {}: {}", i + 1, shard);
    }
    println!();

    match token_store::store(&res.root_token)? {
        token_store::StoredIn::Keyring => println!("Root token stored in the OS keychain."),
        token_store::StoredIn::File(path) => {
            println!(
                "OS keychain unavailable; root token stored in {} (0600).",
                path.display()
            );
        }
    }
    Ok(())
}

fn prompt_init_params() -> Result<InitParams, Box<dyn Error>> {
    let total_shares = prompt::line("Total shares: ")?
        .parse()
        .map_err(|_| "total shares must be a number")?;
    let threshold = prompt::line("Threshold: ")?
        .parse()
        .map_err(|_| "threshold must be a number")?;
    Ok(InitParams {
        total_shares,
        threshold,
    })
}
