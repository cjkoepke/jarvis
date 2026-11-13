use crate::paths::Env;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct InstallRecord {
    pub standards_source: PathBuf,
    pub agent_ids: Vec<String>,
}

impl InstallRecord {
    pub fn load(env: &Env) -> Option<InstallRecord> {
        let text = fs::read_to_string(env.install_record_file()).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self, env: &Env) -> Result<()> {
        let text = serde_json::to_string_pretty(self)? + "\n";
        fs::write(env.install_record_file(), text).context("recording the install")
    }
}
