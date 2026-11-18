use crate::files::find_on_path;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const EXECUTABLE: &str = "ast-grep";
const PACKAGE: &str = "ast-grep";

/// Homebrew pours a prebuilt ast-grep, where cargo compiles one for minutes.
const BY_PREFERENCE: [PackageManager; 2] = [PackageManager::Homebrew, PackageManager::Cargo];

pub fn on_path() -> Option<PathBuf> {
    find_on_path(EXECUTABLE)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Homebrew,
    Cargo,
}

#[derive(Clone, Copy)]
pub enum Action {
    Install,
    Uninstall,
}

impl PackageManager {
    pub fn first_on_path() -> Option<PackageManager> {
        BY_PREFERENCE
            .into_iter()
            .find(|manager| find_on_path(manager.executable()).is_some())
    }

    /// Never waits on a question, and shows the package manager's own progress.
    pub fn run(self, action: Action, home: &Path) -> Result<()> {
        let command_line = self.command_line(action);
        let Some(executable) = find_on_path(self.executable()) else {
            bail!("`{}` is not on PATH; run `{command_line}` where it is", self.executable());
        };
        let finished = Command::new(executable)
            .args(self.arguments(action))
            .envs(self.environment().iter().copied())
            .current_dir(home)
            .stdin(Stdio::null())
            .status()
            .with_context(|| format!("starting `{command_line}`"))?;
        if !finished.success() {
            bail!("`{command_line}` did not finish ({finished})");
        }
        Ok(())
    }

    /// True for a copy the package manager keeps off PATH as well, which is the person's.
    pub fn holds_ast_grep(self) -> bool {
        let Some(executable) = find_on_path(self.executable()) else {
            return false;
        };
        let listed = Command::new(executable)
            .args(self.listing_arguments())
            .envs(self.environment().iter().copied())
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        let names_the_package = |line: &str| line.split_whitespace().next() == Some(PACKAGE);
        listed.is_ok_and(|listed| listed.status.success() && String::from_utf8_lossy(&listed.stdout).lines().any(names_the_package))
    }

    pub fn command_line(self, action: Action) -> String {
        format!("{} {}", self.executable(), self.arguments(action).join(" "))
    }

    fn executable(self) -> &'static str {
        match self {
            PackageManager::Homebrew => "brew",
            PackageManager::Cargo => "cargo",
        }
    }

    /// The crate also builds `sg`, which would hide the group command of that name on Linux.
    fn arguments(self, action: Action) -> &'static [&'static str] {
        match (self, action) {
            (PackageManager::Homebrew, Action::Install) => &["install", "--formula", PACKAGE],
            (PackageManager::Homebrew, Action::Uninstall) => &["uninstall", "--formula", PACKAGE],
            (PackageManager::Cargo, Action::Install) => &["install", "--locked", "--bin", EXECUTABLE, PACKAGE],
            (PackageManager::Cargo, Action::Uninstall) => &["uninstall", "--package", PACKAGE],
        }
    }

    /// Each prints a line that starts with the package's name when it has the package.
    fn listing_arguments(self) -> &'static [&'static str] {
        match self {
            PackageManager::Homebrew => &["list", "--formula", "--versions", PACKAGE],
            PackageManager::Cargo => &["install", "--list"],
        }
    }

    /// Keeps Homebrew from tidying up formulae this run did not name.
    fn environment(self) -> &'static [(&'static str, &'static str)] {
        match self {
            PackageManager::Homebrew => &[("HOMEBREW_NO_AUTOREMOVE", "1"), ("HOMEBREW_NO_INSTALL_CLEANUP", "1")],
            PackageManager::Cargo => &[],
        }
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PackageManager::Homebrew => write!(formatter, "Homebrew"),
            PackageManager::Cargo => write!(formatter, "cargo"),
        }
    }
}
