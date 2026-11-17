use crate::git;
use anyhow::Result;

pub struct PushedRef {
    pub local_ref: String,
    pub local_sha: String,
    pub remote_ref: String,
    pub remote_sha: String,
}

impl PushedRef {
    pub fn parse(stdin_line: &str) -> Option<PushedRef> {
        let fields: Vec<&str> = stdin_line.split_whitespace().collect();
        let [local_ref, local_sha, remote_ref, remote_sha] = fields[..] else {
            return None;
        };
        Some(PushedRef {
            local_ref: local_ref.to_string(),
            local_sha: local_sha.to_string(),
            remote_ref: remote_ref.to_string(),
            remote_sha: remote_sha.to_string(),
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

/// The pushed refs that have commits the remote lacks, by their name on the remote, and
/// the tip of each.
pub struct Outgoing {
    pub remote: String,
    /// `None` for a push to a URL, which names no configured remote.
    pub remote_name: Option<String>,
    pub refs: Vec<String>,
    pub commits: Vec<String>,
}

fn is_no_commit(sha: &str) -> bool {
    sha.bytes().all(|byte| byte == b'0')
}
