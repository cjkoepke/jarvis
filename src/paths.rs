use crate::origin;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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
        let started_as = std::env::current_exe().context("locating the jarvis binary")?;
        Ok(Env {
            home,
            jarvis_executable: origin::lasting_executable(&started_as),
        })
    }

    pub fn jarvis_dir(&self) -> PathBuf {
        self.home.join(".jarvis")
    }

    pub fn installed_standards_dir(&self) -> PathBuf {
        self.jarvis_dir().join("standards")
    }

    /// Where an install puts the standards together before they replace the installed ones.
    pub fn next_standards_dir(&self) -> PathBuf {
        self.jarvis_dir().join("standards.next")
    }

    /// A person's own core files and skills, which no command overwrites.
    pub fn custom_standards_dir(&self) -> PathBuf {
        self.jarvis_dir().join("custom")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.jarvis_dir().join("settings.toml")
    }

    pub fn refused_pushes_file(&self) -> PathBuf {
        self.jarvis_dir().join("refused-pushes.json")
    }

    pub fn install_record_file(&self) -> PathBuf {
        self.jarvis_dir().join("install.json")
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

    /// The jarvis executable `command` runs, when it is `subcommand` aimed at this home.
    pub fn executable_in(&self, command: &str, subcommand: &str) -> Option<PathBuf> {
        let aimed_at_this_home = format!(" --home {} {subcommand}", single_quoted(&self.home));
        let quoted_executable = command.strip_suffix(&aimed_at_this_home)?;
        let executable = quoted_executable.strip_prefix('\'')?.strip_suffix('\'')?;
        Some(PathBuf::from(executable.replace(ESCAPED_QUOTE, "'")))
    }
}

const ESCAPED_QUOTE: &str = r"'\''";

fn single_quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', ESCAPED_QUOTE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registered_command_gives_back_its_executable_only_for_its_own_subcommand() {
        let home = tempfile::tempdir().unwrap();
        let env = Env {
            home: home.path().canonicalize().unwrap(),
            jarvis_executable: PathBuf::from("/opt/it's here/jarvis"),
        };
        let registered = env.shell_command("hook claude SessionStart");

        assert_eq!(
            env.executable_in(&registered, "hook claude SessionStart"),
            Some(env.jarvis_executable.clone())
        );
        assert_eq!(env.executable_in(&registered, "pre-push"), None);
    }
}
