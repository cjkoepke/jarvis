use crate::git;
use crate::rules::FileFix;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::Command;

const IN_PROGRESS: &str = "JARVIS_FIX_COMMIT";
pub const MESSAGE: &str = "Apply jarvis standards fixes";

pub fn in_progress() -> bool {
    std::env::var_os(IN_PROGRESS).is_some()
}

/// Returns the short id of the new commit.
///
/// @agent Naming the paths makes git commit only those files, so anything else the
/// person has staged stays staged. `--no-verify` and the `IN_PROGRESS` variable
/// keep this commit from triggering hooks, jarvis included.
pub fn create(fixes: &[FileFix]) -> Result<String> {
    let repo_root = PathBuf::from(git::run(&["rev-parse", "--show-toplevel"])?);
    for fix in fixes {
        std::fs::write(repo_root.join(&fix.path), &fix.contents).with_context(|| format!("writing {}", fix.path))?;
    }

    let mut commit = Command::new("git");
    commit.current_dir(&repo_root).env(IN_PROGRESS, "1");
    commit
        .args(["commit", "--quiet", "--no-verify", "-m", MESSAGE, "--"])
        .args(fixes.iter().map(|fix| &fix.path));
    let output = commit.output().context("running git commit")?;
    if !output.status.success() {
        bail!("could not commit the fixes: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    git::run(&["rev-parse", "--short", "HEAD"])
}
