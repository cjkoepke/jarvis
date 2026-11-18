use crate::ast_grep::PackageManager;
use crate::paths::Env;
use crate::standards::source::Source;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct InstallRecord {
    /// The folder named with `--standards`. None stands for the packaged standards.
    #[serde(default)]
    pub standards_source: Option<PathBuf>,
    pub agent_ids: Vec<String>,
    /// Every package manager an install put ast-grep on the machine with, which `uninstall`
    /// removes it with. Empty when ast-grep was already there.
    #[serde(default)]
    pub ast_grep_installed_by: Vec<PackageManager>,
}

impl InstallRecord {
    pub fn load(env: &Env) -> Option<InstallRecord> {
        let text = fs::read_to_string(env.install_record_file()).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn standards(&self) -> Source {
        match &self.standards_source {
            Some(folder) => Source::Folder(folder.clone()),
            None => Source::Packaged,
        }
    }

    /// The standards a bare install takes: those of the last install, or the packaged
    /// ones when there was none or its folder is gone.
    pub fn standards_in_effect(last_install: Option<&InstallRecord>) -> Source {
        let last_source = last_install.map(InstallRecord::standards);
        let still_there = |source: &Source| source.folder().is_none_or(|folder| folder.is_dir());
        last_source.filter(still_there).unwrap_or(Source::Packaged)
    }

    pub fn save(&self, env: &Env) -> Result<()> {
        let text = serde_json::to_string_pretty(self)? + "\n";
        fs::write(env.install_record_file(), text).context("recording the install")
    }
}
