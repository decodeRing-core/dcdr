use std::env;

use decodering_core::plugin::error::PluginError;
use decodering_core::plugin::orchestrator::Orchestrator;
use dotenvy::dotenv;

fn main() -> Result<(), PluginError> {
    println!("decodeRing CLI starting...");
    dotenv().ok();
    let plugin_directory = env::var("PLUGIN_DIRECTORY").expect("PLUGIN_DIRECTORY must be set");

    let mut orchestrator = Orchestrator::new();
    orchestrator.load_wasm_plugins_from_dir(&plugin_directory)?;

    println!("decodeRing core ready. Accepting OSL requests.");

    println!("decodeRing exiting.");
    Ok(())
}
