use clap::{Parser, Subcommand};
#[cfg(feature = "tpm")]
use std::path::PathBuf;
use std::process::ExitCode;

use crate::cmd::app::AppCommand;
use crate::cmd::osl::OslCommand;
use crate::cmd::raft::RaftCommand;
use crate::cmd::system::SystemCommand;

mod api;
mod cmd;
mod output;
mod progress;
mod prompt;
mod session;
mod source;
mod state;
mod token_store;

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

    #[cfg(feature = "aws")]
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

    /// Generate TPM 2.0 attestation parameters for enrolling this machine
    #[cfg(feature = "tpm")]
    TpmParams {
        /// Emit progress messages to stderr
        #[arg(long, short = 'd')]
        debug: bool,
        #[arg(long, short = 'o', value_name = "FILE")]
        out: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let Args { addr, command } = Args::parse();

    let dispatch = Box::pin(async move {
        match command {
            Command::GenerateSchema => cmd::schema::run(),
            #[cfg(feature = "aws")]
            Command::AwsSig { region } => cmd::aws_sig::run(&region).await,
            Command::System(cmd) => cmd::system::run(cmd, &addr).await,
            Command::Raft(cmd) => cmd::raft::run(cmd, &addr).await,
            Command::App(cmd) => cmd::app::run(cmd, &addr).await,
            Command::Osl(cmd) => cmd::osl::run(cmd, &addr).await,
            #[cfg(feature = "tpm")]
            Command::TpmParams { out, debug } => cmd::tpm_params::run(&out, debug),
        }
    });

    let result = session::frame("decodering CLI v0.1", dispatch).await;
    token_store::release();

    if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
