#![allow(clippy::print_stdout)]
use std::error::Error;
use std::io::{self, Write};

pub fn line(label: &str) -> Result<String, Box<dyn Error>> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_owned())
}
