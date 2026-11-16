use crate::git;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

const GPG_CONTROL: &str = "gpgconf";
const SSH_SIGNING: &str = "ssh";
const GIT_DIR_OF_A_CHECKOUT: &str = ".git";

/// The repository a command runs in, as an agent working there needs to know it.
pub struct Repo {
    /// The top folder of the checkout, which is where git starts a hook and so the reviewer.
    /// A bare repository has no checkout, so there it is the folder the command ran in.
    pub root: PathBuf,
    /// Where git keeps this checkout's state. A linked worktree has two such folders.
    pub git_dirs: Vec<PathBuf>,
    pub signs_commits: bool,
}

impl Repo {
    pub fn current() -> Result<Repo> {
        let git_dirs = git::run(&["rev-parse", "--path-format=absolute", "--git-dir", "--git-common-dir"])?;
        let mut git_dirs: Vec<PathBuf> = git_dirs.lines().map(|dir| without_symlinks(Path::new(dir))).collect();
        git_dirs.dedup();
        let root = match git::run(&["rev-parse", "--show-toplevel"]) {
            Ok(top_folder) => PathBuf::from(top_folder),
            Err(_) => std::env::current_dir()?,
        };
        let signs_commits = git::run(&["config", "--type=bool", "--get", "commit.gpgsign"]).is_ok_and(|signs| signs == "true");
        Ok(Repo {
            root: without_symlinks(&root),
            git_dirs,
            signs_commits,
        })
    }

    /// The checkout a linked worktree was added from, which is where both agents record
    /// trust for the worktree. A worktree of a bare repository has none.
    pub fn main_checkout(&self) -> Option<PathBuf> {
        let shared = self.shared_git_dir()?;
        let belongs_to_a_checkout = shared.file_name().is_some_and(|name| name == GIT_DIR_OF_A_CHECKOUT);
        shared.parent().filter(|_| belongs_to_a_checkout).map(Path::to_path_buf)
    }

    /// The git folder a linked worktree shares with the repository it was added from.
    pub fn shared_git_dir(&self) -> Option<&Path> {
        match &self.git_dirs[..] {
            [_own, shared] => Some(shared),
            _ => None,
        }
    }

    /// A sandboxed reviewer can reach a running gpg-agent but cannot start one.
    pub fn start_signing_agent(&self) {
        let signs_with_ssh = git::run(&["config", "--get", "gpg.format"]).is_ok_and(|format| format == SSH_SIGNING);
        if self.signs_commits && !signs_with_ssh {
            Command::new(GPG_CONTROL).args(["--launch", "gpg-agent"]).output().ok();
        }
    }
}

fn without_symlinks(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_with_git_dirs(git_dirs: &[&str]) -> Repo {
        Repo {
            root: PathBuf::from("/work/checkout"),
            git_dirs: git_dirs.iter().map(PathBuf::from).collect(),
            signs_commits: false,
        }
    }

    #[test]
    fn only_a_worktree_linked_to_a_checkout_has_a_main_checkout() {
        let own_checkout = repo_with_git_dirs(&["/work/checkout/.git"]);
        let linked_worktree = repo_with_git_dirs(&["/work/main/.git/worktrees/topic", "/work/main/.git"]);
        let worktree_of_a_bare_repo = repo_with_git_dirs(&["/work/bare.git/worktrees/topic", "/work/bare.git"]);

        assert_eq!(own_checkout.main_checkout(), None);
        assert_eq!(linked_worktree.main_checkout(), Some(PathBuf::from("/work/main")));
        assert_eq!(worktree_of_a_bare_repo.main_checkout(), None);
        assert_eq!(worktree_of_a_bare_repo.shared_git_dir(), Some(Path::new("/work/bare.git")));
    }
}
