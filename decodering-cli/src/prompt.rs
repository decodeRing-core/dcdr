use std::error::Error;
use std::io::{self, Write};

#[allow(clippy::print_stdout)]
pub fn line(label: &str) -> Result<String, Box<dyn Error>> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_owned())
}

pub fn required(label: &str) -> Result<String, Box<dyn Error>> {
    let value = line(label)?;
    if value.is_empty() {
        let field = label.trim().trim_end_matches(':').trim();
        return Err(format!("{field} is required").into());
    }
    Ok(value)
}

pub fn or_default(value: String, default: &str) -> String {
    if value.is_empty() {
        default.to_owned()
    } else {
        value
    }
}
