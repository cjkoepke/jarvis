use crate::files;
use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

pub type Config = Map<String, Value>;

/// A missing or empty file reads as an empty config. Anything else that is not a JSON
/// object is an error, so a file jarvis could not read is never overwritten.
pub fn read(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::new());
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Config::new());
    }
    match serde_json::from_str(&text).with_context(|| format!("{} is not valid JSON", path.display()))? {
        Value::Object(config) => Ok(config),
        _ => bail!("{} does not hold a JSON object", path.display()),
    }
}

pub fn write(path: &Path, config: &Config) -> Result<()> {
    files::replace(path, &(serde_json::to_string_pretty(config)? + "\n"))
}
