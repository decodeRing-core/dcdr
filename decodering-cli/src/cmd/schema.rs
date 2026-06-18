use decodering_core::plugin::osl_contract::Capability;
use decodering_core::plugin::osl_contract::DeleteInput;
use decodering_core::plugin::osl_contract::DeleteOutput;
use decodering_core::plugin::osl_contract::DescribeInput;
use decodering_core::plugin::osl_contract::DescribeOutput;
use decodering_core::plugin::osl_contract::DestroyInput;
use decodering_core::plugin::osl_contract::DestroyOutput;
use decodering_core::plugin::osl_contract::ReadInput;
use decodering_core::plugin::osl_contract::ReadOutput;
use decodering_core::plugin::osl_contract::RestoreInput;
use decodering_core::plugin::osl_contract::RestoreOutput;
use decodering_core::plugin::osl_contract::WriteInput;
use decodering_core::plugin::osl_contract::WriteOutput;
use schemars::schema_for;
use std::io::Write;
use std::{error::Error, fs, path::Path};

pub fn run() -> Result<(), Box<dyn Error>> {
    let out = Path::new("schema");
    if out.exists() {
        fs::remove_dir_all(out)?;
    }
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
    emit!(ReadOutput, "read_output");
    emit!(WriteInput, "write_input");
    emit!(WriteOutput, "write_output");
    emit!(DeleteInput, "delete_input");
    emit!(DeleteOutput, "delete_output");
    emit!(DestroyInput, "destroy_input");
    emit!(DestroyOutput, "destroy_output");
    emit!(RestoreInput, "restore_input");
    emit!(RestoreOutput, "restore_output");
    emit!(DescribeInput, "describe_input");
    emit!(DescribeOutput, "describe_output");

    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "Wrote schemas to ./{}/", out.display())?;
    Ok(())
}
