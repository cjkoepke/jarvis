use crate::git;
use anyhow::Result;
use std::path::PathBuf;

/// The repository a push comes from, as a fixing reviewer needs to know it.
pub struct Repo {
    pub dir: PathBuf,
    /// Where git keeps this checkout's state. A linked worktree has two such folders.
    pub git_dirs: Vec<PathBuf>,
    pub signs_commits: bool,
}

impl Repo {
    pub fn current() -> Result<Repo> {
        let dir = git::run(&["rev-parse", "--show-toplevel"])?;
        let git_dirs = git::run(&["rev-parse", "--path-format=absolute", "--git-dir", "--git-common-dir"])?;
        let mut git_dirs: Vec<PathBuf> = git_dirs.lines().map(PathBuf::from).collect();
        git_dirs.dedup();
        let signs_commits = git::run(&["config", "--type=bool", "--get", "commit.gpgsign"]).is_ok_and(|signs| signs == "true");
        Ok(Repo {
            dir: PathBuf::from(dir),
            git_dirs,
            signs_commits,
        })
    }
}
