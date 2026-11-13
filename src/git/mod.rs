pub mod hook_config;
pub mod outgoing;

use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn run(args: &[&str]) -> Result<String> {
    Ok(String::from_utf8_lossy(&run_bytes(args)?).trim_end().to_string())
}

pub fn run_bytes(args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git").args(args).output().context("running git")?;
    if !output.status.success() {
        bail!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(output.stdout)
}
