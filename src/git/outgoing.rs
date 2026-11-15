use crate::git;
use anyhow::Result;

pub struct PushedRef {
    pub local_sha: String,
}

impl PushedRef {
    pub fn parse(stdin_line: &str) -> Option<PushedRef> {
        let fields: Vec<&str> = stdin_line.split_whitespace().collect();
        let [_local_ref, local_sha, _remote_ref, _remote_sha] = fields[..] else {
            return None;
        };
        Some(PushedRef {
            local_sha: local_sha.to_string(),
        })
    }

    pub fn deletes_the_ref(&self) -> bool {
        is_no_commit(&self.local_sha)
    }

    pub fn has_commits_the_remote_lacks(&self, remote: &str) -> Result<bool> {
        let newest = git::run(&[
            "rev-list",
            "--max-count=1",
            &self.local_sha,
            "--not",
            &format!("--remotes={remote}"),
        ])?;
        Ok(!newest.is_empty())
    }
}

/// The tip of each pushed ref that has commits the remote lacks.
pub struct Outgoing {
    pub remote: String,
    pub commits: Vec<String>,
}

fn is_no_commit(sha: &str) -> bool {
    sha.bytes().all(|byte| byte == b'0')
}
