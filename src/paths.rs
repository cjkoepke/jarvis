use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// @agent Everything jarvis reads or writes hangs off `home`, so `--home` can point
/// a whole install at a throwaway directory. Never resolve a path from `$HOME` directly.
#[derive(Debug, Clone)]
pub struct Env {
    pub home: PathBuf,
    pub jarvis_executable: PathBuf,
}

impl Env {
    pub fn new(home_flag: Option<PathBuf>) -> Result<Env> {
        let home = match home_flag {
            Some(home) => home,
            None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set; pass --home")?),
        };
        let home = home
            .canonicalize()
            .with_context(|| format!("home directory {} does not exist", home.display()))?;
        let jarvis_executable = std::env::current_exe().context("locating the jarvis binary")?;
        Ok(Env { home, jarvis_executable })
    }

    pub fn jarvis_dir(&self) -> PathBuf {
        self.home.join(".jarvis")
    }

    pub fn installed_standards_dir(&self) -> PathBuf {
        self.jarvis_dir().join("standards")
    }

    pub fn installed_agents_file(&self) -> PathBuf {
        self.jarvis_dir().join("agents")
    }

    pub fn global_gitconfig(&self) -> PathBuf {
        self.home.join(".gitconfig")
    }

    /// A `/bin/sh` command line that runs this jarvis binary against this home.
    pub fn shell_command(&self, subcommand: &str) -> String {
        format!(
            "{} --home {} {subcommand}",
            single_quoted(&self.jarvis_executable),
            single_quoted(&self.home)
        )
    }
}

fn single_quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}
