use clap::{Args, Subcommand};
use serde::Deserialize;
use std::error::Error;

use crate::api::{self, InitRequest, PluginsCredentials};
use crate::source::{SecretSource, ValueSource};
use crate::token_store::{self, StoredIn};

#[derive(Subcommand)]
pub enum SystemCommand {
    /// Initialize the system: generates Shamir shards and a root key
    Init(InitInput),
}

#[derive(Args)]
pub struct InitInput {
    /// Non-secret params: inline JSON string or `@file`
    #[arg(long, value_name = "SOURCE")]
    params: ValueSource,

    /// Optional plugin credentials from a file (`@path`) or stdin (`-`)
    #[arg(long, value_name = "FILE|-")]
    plugins_credentials: Option<SecretSource>,

    /// Print the root key instead of storing it
    #[arg(long)]
    no_store: bool,
}

#[derive(Deserialize)]
struct InitParams {
    total_shares: u8,
    threshold: u8,
}

pub async fn run(cmd: SystemCommand) -> Result<(), Box<dyn Error>> {
    match cmd {
        SystemCommand::Init(input) => init(input).await,
    }
}

#[allow(clippy::print_stdout)]
async fn init(input: InitInput) -> Result<(), Box<dyn Error>> {
    let params: InitParams = serde_json::from_str(&input.params.read()?)?;
    if params.threshold == 0 || params.threshold > params.total_shares {
        return Err("threshold must be between 1 and total_shares".into());
    }

    let plugins_credentials: Option<PluginsCredentials> = match &input.plugins_credentials {
        Some(src) => Some(serde_json::from_str(&src.read()?)?),
        None => None,
    };

    let res = api::init(InitRequest {
        total_shares: params.total_shares,
        threshold: params.threshold,
        plugins_credentials,
    })
    .await?;

    println!("System initialized.\n");
    println!("Unseal shards — distribute these to separate operators.");
    println!("They are shown ONCE and are not stored anywhere:\n");
    for (i, shard) in res.shards.iter().enumerate() {
        println!("  Shard {}: {}", i + 1, shard);
    }
    println!(
        "\nAny {} of {} shards are required to unlock the system.\n",
        params.threshold, params.total_shares
    );

    if input.no_store {
        println!("Root key (not stored): {}", res.root_key);
    } else {
        match token_store::store(&res.root_key)? {
            StoredIn::Keyring => println!("Root key stored in the OS keychain."),
            StoredIn::File(path) => println!(
                "OS keychain unavailable; root key stored in {} (0600).",
                path.display()
            ),
        }
    }

    Ok(())
}
