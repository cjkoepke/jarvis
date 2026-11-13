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

    /// The jarvis executable `command` runs, when it is `subcommand` aimed at this home.
    ///
    /// @agent Deliberately accepts any jarvis binary. Hooks registered by the
    /// installed binary must still count when `status` runs from another build.
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
