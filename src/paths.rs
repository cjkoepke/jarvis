use anyhow::{Context, Result};
use std::ffi::CStr;
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

    /// @agent Compares against the account database instead of `$HOME`, so overriding
    /// `HOME` cannot slip a write to the real home past the `--real` guard.
    pub fn is_real_home(&self) -> bool {
        match os_account_home() {
            Some(real_home) => real_home.canonicalize().unwrap_or(real_home) == self.home,
            None => true,
        }
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

/// @agent SAFETY: `getpwuid` returns null or a pointer into static storage. The
/// string is copied out before returning and the pointer is not kept.
fn os_account_home() -> Option<PathBuf> {
    unsafe {
        let account = libc::getpwuid(libc::getuid());
        if account.is_null() || (*account).pw_dir.is_null() {
            return None;
        }
        Some(PathBuf::from(CStr::from_ptr((*account).pw_dir).to_string_lossy().into_owned()))
    }
}

fn single_quoted(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_at(home: PathBuf) -> Env {
        Env {
            home: home.canonicalize().unwrap(),
            jarvis_executable: PathBuf::new(),
        }
    }

    #[test]
    fn only_the_os_account_home_counts_as_the_real_home() {
        let throwaway = tempfile::tempdir().unwrap();
        assert!(!env_at(throwaway.path().to_path_buf()).is_real_home());
        assert!(env_at(os_account_home().unwrap()).is_real_home());
    }
}
