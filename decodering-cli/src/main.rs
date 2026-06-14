use clap::{Parser, Subcommand};
use std::error::Error;
use std::io::Write;

use crate::aws_sig::generate_aws_sig;
use crate::raft::RaftCommand;
use crate::schema::generate_schema;
use crate::system::SystemCommand;

mod api;
mod aws_sig;
mod output;
mod prompt;
mod raft;
mod schema;
mod source;
mod system;
mod token_store;
#[cfg(feature = "tpm")]
mod tpm_params;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        long,
        global = true,
        env = "DCDR_ADDR",
        default_value = "http://127.0.0.1:21001"
    )]
    addr: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate the JSON schema
    GenerateSchema,

    /// Generate a signed AWS STS `GetCallerIdentity` request and print it as JSON
    AwsSig {
        /// AWS region to sign for
        #[arg(long, default_value = "us-east-1")]
        region: String,
    },

    #[command(subcommand)]
    System(SystemCommand),

    #[command(subcommand)]
    Raft(RaftCommand),

    #[cfg(feature = "tpm")]
    TpmParams {
        /// Emit progress messages to stderr
        #[arg(long, short = 'd')]
        debug: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let result = match args.command {
        Command::GenerateSchema => generate_schema(),
        Command::AwsSig { region } => generate_aws_sig(&region).await,
        #[cfg(feature = "tpm")]
        Command::TpmParams { debug } => todo!(),
        Command::System(cmd) => system::run(cmd, &args.addr).await,
        Command::Raft(cmd) => raft::run(cmd, &args.addr).await,
    };
    token_store::release();

    if let Err(e) = result {
        let _ = writeln!(std::io::stderr(), "Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}
