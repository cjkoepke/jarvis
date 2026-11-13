use crate::git;
use anyhow::Result;

pub struct PushedRef {
    pub local_sha: String,
    pub remote_sha: String,
}

impl PushedRef {
    /// @agent Git gives a pre-push hook one line per ref on stdin:
    /// `<local ref> <local sha> <remote ref> <remote sha>`. An all-zero sha means
    /// "no commit": a deleted ref locally, a new ref on the remote.
    pub fn parse(stdin_line: &str) -> Option<PushedRef> {
        let fields: Vec<&str> = stdin_line.split_whitespace().collect();
        let [_local_ref, local_sha, _remote_ref, remote_sha] = fields[..] else {
            return None;
        };
        Some(PushedRef {
            local_sha: local_sha.to_string(),
            remote_sha: remote_sha.to_string(),
        })
    }

    pub fn deletes_the_ref(&self) -> bool {
        is_no_commit(&self.local_sha)
    }
}

pub struct Outgoing {
    pub commit_messages: String,
    pub diff: String,
}

/// Returns `None` when the remote already has every commit of the pushed ref.
pub fn commits_the_remote_lacks(pushed: &PushedRef, remote: Option<&str>) -> Result<Option<Outgoing>> {
    let missing_from_remote = revisions_missing_from_remote(pushed, remote);
    let missing_from_remote: Vec<&str> = missing_from_remote.iter().map(String::as_str).collect();

    let newest_first = git::run(&[&["rev-list"], &missing_from_remote[..]].concat())?;
    let Some(oldest_missing_commit) = newest_first.lines().last() else {
        return Ok(None);
    };
    let base = parent_or_empty_tree(oldest_missing_commit)?;

    Ok(Some(Outgoing {
        commit_messages: git::run(&[&["log", "--format=commit %h%n%B"], &missing_from_remote[..]].concat())?,
        diff: git::run(&["diff", "--no-color", "--no-ext-diff", &base, &pushed.local_sha])?,
    }))
}

fn revisions_missing_from_remote(pushed: &PushedRef, remote: Option<&str>) -> Vec<String> {
    let on_remote_branches = match remote {
        Some(remote) => format!("--remotes={remote}"),
        None => "--remotes".to_string(),
    };
    let mut revisions = vec![pushed.local_sha.clone(), "--not".to_string(), on_remote_branches];
    if remote_commit_is_known_locally(&pushed.remote_sha) {
        revisions.push(pushed.remote_sha.clone());
    }
    revisions
}

fn remote_commit_is_known_locally(remote_sha: &str) -> bool {
    !is_no_commit(remote_sha) && git::run(&["cat-file", "-e", remote_sha]).is_ok()
}

fn parent_or_empty_tree(commit: &str) -> Result<String> {
    let parent = git::run(&["rev-parse", "--verify", "--quiet", &format!("{commit}^")]);
    parent.or_else(|_root_commit| git::run(&["hash-object", "-t", "tree", "/dev/null"]))
}

fn is_no_commit(sha: &str) -> bool {
    sha.bytes().all(|byte| byte == b'0')
}
