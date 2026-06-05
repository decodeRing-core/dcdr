use clap::{Parser, Subcommand};
use std::error::Error;

use crate::{aws_sig::generate_aws_sig, schema::generate_schema};

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
    // TpmParams {
    //     /// Emit progress messages to stderr
    //     #[arg(long, short = 'd')]
    //     debug: bool,
    // },
}

mod aws_sig;
mod schema;
mod tpm_params;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command {
        Command::GenerateSchema => generate_schema()?,
        Command::AwsSig { region } => generate_aws_sig(&region).await?,
        //Command::TpmParams { debug } => todo!(),
    }

    Ok(())
}
