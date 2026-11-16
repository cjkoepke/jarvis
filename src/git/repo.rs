use crate::git;
use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

const GPG_CONTROL: &str = "gpgconf";
const SSH_SIGNING: &str = "ssh";

/// The repository a push comes from, as a fixing reviewer needs to know it.
pub struct Repo {
    /// Where git keeps this checkout's state. A linked worktree has two such folders.
    pub git_dirs: Vec<PathBuf>,
    pub signs_commits: bool,
}

impl Repo {
    pub fn current() -> Result<Repo> {
        let git_dirs = git::run(&["rev-parse", "--path-format=absolute", "--git-dir", "--git-common-dir"])?;
        let mut git_dirs: Vec<PathBuf> = git_dirs.lines().map(PathBuf::from).collect();
        git_dirs.dedup();
        let signs_commits = git::run(&["config", "--type=bool", "--get", "commit.gpgsign"]).is_ok_and(|signs| signs == "true");
        Ok(Repo { git_dirs, signs_commits })
    }

    /// A sandboxed reviewer can reach a running gpg-agent but cannot start one.
    pub fn start_signing_agent(&self) {
        let signs_with_ssh = git::run(&["config", "--get", "gpg.format"]).is_ok_and(|format| format == SSH_SIGNING);
        if self.signs_commits && !signs_with_ssh {
            Command::new(GPG_CONTROL).args(["--launch", "gpg-agent"]).output().ok();
        }
    }
}
