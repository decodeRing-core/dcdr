use clap::{Parser, Subcommand};
use std::error::Error;

use crate::aws_sig::generate_aws_sig;
use crate::schema::generate_schema;
use crate::system::SystemCommand;

mod api;
mod aws_sig;
mod schema;
mod source;
mod system;
mod token_store;
#[cfg(feature = "tpm")]
mod tpm_params;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
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

    match args.command {
        Command::GenerateSchema => generate_schema()?,
        Command::AwsSig { region } => generate_aws_sig(&region).await?,
        #[cfg(feature = "tpm")]
        Command::TpmParams { debug } => todo!(),
        Command::System(cmd) => system::run(cmd).await?,
    }
    token_store::release();

    Ok(())
}
