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
    /// Update plugin credentials
    PluginConfig(PluginConfigInput),
}

#[derive(Args)]
pub struct PluginConfigInput {
    /// Plugin credentials from a file (`@path`) or stdin (`-`). Omit for interactive entry.
    #[arg(long, value_name = "FILE|-")]
    plugins_credentials: Option<SecretSource>,
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
        SystemCommand::PluginConfig(input) => plugin_config(input, addr).await,
    }
}

async fn plugin_config(input: PluginConfigInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let plugins_credentials: PluginsCredentials = match &input.plugins_credentials {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => build_plugin_credentials()?,
    };

    let token = token_store::load()?.ok_or("no root token found; run `system init` first")?;

    let resp = api::system_plugin_config(
        addr,
        &token,
        api::PluginConfigRequest {
            plugins_credentials,
        },
    )
    .await?;
    output::report(&resp);
    Ok(())
}

async fn unlock(input: UnlockInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let shards = match &input.params {
        Some(src) => serde_json::from_str::<api::UnlockRequest>(&src.read()?)?.shards,
        None => prompt_shards()?,
    };
    if shards.is_empty() {
        return Err("at least one shard is required".into());
    }
    let resp = api::system_unlock(addr, api::UnlockRequest { shards }).await?;
    output::report(&resp);
    Ok(())
}

async fn status(addr: &str) -> Result<(), Box<dyn Error>> {
    let resp = api::system_status(addr).await?;
    output::report(&resp);
    Ok(())
}

async fn init(input: InitInput, addr: &str) -> Result<(), Box<dyn Error>> {
    let interactive = input.params.is_none();

    let params: InitParams = match &input.params {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None => prompt_init_params()?,
    };
    if params.threshold == 0 || params.threshold > params.total_shares {
        return Err("threshold must be between 1 and total_shares".into());
    }

    let plugins_credentials: PluginsCredentials = match &input.plugins_credentials {
        Some(src) => serde_json::from_str(&src.read()?)?,
        None if interactive => prompt_plugins_credentials()?,
        None => PluginsCredentials::new(),
    };

    if input.with_raft {
        let resp = api::raft_init(addr, api::RaftInitRequest { raft_init: vec![] }).await?;
        output::report(&resp);
    }

    let res = api::system_init(
        addr,
        api::InitRequest {
            total_shares: params.total_shares,
            threshold: params.threshold,
            plugins_credentials,
        },
    )
    .await?;

    let _ = cliclack::log::success("System initialized.");

    let body = res
        .shards
        .iter()
        .enumerate()
        .map(|(i, shard)| format!("{}. {shard}", i + 1))
        .collect::<Vec<String>>()
        .join("\n");
    let _ = cliclack::note(
        "Unseal shards — distribute to separate operators (shown once)",
        body,
    );

    match token_store::store(&res.root_token)? {
        token_store::StoredIn::Keyring => {
            let _ = cliclack::log::success("Root token stored in the OS keychain.");
        }
        token_store::StoredIn::File(path) => {
            let _ = cliclack::log::warning(format!(
                "OS keychain unavailable; root token stored in {} (0600).",
                path.display()
            ));
        }
    }
    Ok(())
}

fn prompt_plugins_credentials() -> Result<PluginsCredentials, Box<dyn Error>> {
    if !prompt::confirm("Add plugin credentials?")? {
        return Ok(PluginsCredentials::new());
    }
    build_plugin_credentials()
}

fn build_plugin_credentials() -> Result<PluginsCredentials, Box<dyn Error>> {
    let mut creds = PluginsCredentials::new();
    loop {
        let backend = prompt::required("Plugin ref (e.g. openbao-rs)")?;
        let mut fields = serde_json::Map::new();
        loop {
            let key = prompt::line("Credential field (empty to finish)")?;
            if key.is_empty() {
                break;
            }
            let value = prompt::password("Value")?;
            fields.insert(key, serde_json::Value::String(value));
        }
        creds.insert(backend, serde_json::Value::Object(fields));
        if !prompt::confirm("Add another plugin?")? {
            break;
        }
    }
    Ok(creds)
}

fn prompt_init_params() -> Result<InitParams, Box<dyn Error>> {
    Ok(InitParams {
        total_shares: prompt::parse("Total shares")?,
        threshold: prompt::parse("Threshold")?,
    })
}

fn prompt_shards() -> Result<Vec<String>, Box<dyn Error>> {
    let mut shards = Vec::new();
    loop {
        shards.push(prompt::password(&format!("Shard {}", shards.len() + 1))?);
        if !prompt::confirm("Add another shard?")? {
            break;
        }
    }
    Ok(shards)
}
