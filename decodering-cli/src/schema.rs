#![allow(clippy::print_stderr)]

use decodering_core::plugin::osl_contract::Capability;
use decodering_core::plugin::osl_contract::DeleteInput;
use decodering_core::plugin::osl_contract::DeleteOutput;
use decodering_core::plugin::osl_contract::DestroyInput;
use decodering_core::plugin::osl_contract::DestroyOutput;
use decodering_core::plugin::osl_contract::ReadInput;
use decodering_core::plugin::osl_contract::ReadResponse;
use decodering_core::plugin::osl_contract::RestoreInput;
use decodering_core::plugin::osl_contract::RestoreOutput;
use decodering_core::plugin::osl_contract::WriteInput;
use decodering_core::plugin::osl_contract::WriteOutput;
use schemars::schema_for;
use std::{error::Error, fs, path::Path};

pub fn generate_schema() -> Result<(), Box<dyn Error>> {
    let out = Path::new("schema");
    fs::create_dir_all(out)?;

    macro_rules! emit {
        ($ty:ty, $name:expr) => {{
            let schema = schema_for!($ty);
            let json = serde_json::to_string_pretty(&schema).unwrap();
            fs::write(out.join(concat!($name, ".json")), json)?;
        }};
    }

    emit!(Capability, "capability");
    emit!(ReadInput, "read_input");
    emit!(ReadResponse, "read_response");
    emit!(WriteInput, "write_input");
    emit!(WriteOutput, "write_output");
    emit!(DeleteInput, "delete_input");
    emit!(DeleteOutput, "delete_output");
    emit!(DestroyInput, "destroy_input");
    emit!(DestroyOutput, "destroy_output");
    emit!(RestoreInput, "restore_input");
    emit!(RestoreOutput, "restore_output");

    eprintln!("wrote schemas to {}/", out.display());
    Ok(())
}
