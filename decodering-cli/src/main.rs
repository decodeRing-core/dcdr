use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::app::AppCommand;
use crate::aws_sig::generate_aws_sig;
use crate::osl::OslCommand;
use crate::raft::RaftCommand;
use crate::schema::generate_schema;
use crate::system::SystemCommand;

mod api;
mod app;
mod aws_sig;
mod osl;
mod output;
mod progress;
mod prompt;
mod raft;
mod schema;
mod session;
mod source;
mod state;
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

    /// Initialize, unlock, and manage the system lifecycle
    #[command(subcommand)]
    System(SystemCommand),

    /// Initialize and operate the raft cluster
    #[command(subcommand)]
    Raft(RaftCommand),

    /// Manage apps, users, and their credentials
    #[command(subcommand)]
    App(AppCommand),

    /// Read and manage secrets via the OSL API
    #[command(subcommand)]
    Osl(OslCommand),

    /// Generate TPM parameters
    #[cfg(feature = "tpm")]
    TpmParams {
        /// Emit progress messages to stderr
        #[arg(long, short = 'd')]
        debug: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let Args { addr, command } = Args::parse();

    let dispatch = Box::pin(async move {
        match command {
            Command::GenerateSchema => generate_schema(),
            Command::AwsSig { region } => generate_aws_sig(&region).await,
            Command::System(cmd) => system::run(cmd, &addr).await,
            Command::Raft(cmd) => raft::run(cmd, &addr).await,
            Command::App(cmd) => app::run(cmd, &addr).await,
            Command::Osl(cmd) => osl::run(cmd, &addr).await,
            #[cfg(feature = "tpm")]
            Command::TpmParams { debug } => tpm_params::run(debug),
        }
    });

    let result = session::frame("decodering CLI", dispatch).await;
    token_store::release();

    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
