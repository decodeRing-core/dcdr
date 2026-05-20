use clap::Parser;
use std::error::Error;

use crate::schema::generate_schema;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    generate_schema: bool,
}

mod schema;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.generate_schema {
        generate_schema()?;
    }

    Ok(())
}
