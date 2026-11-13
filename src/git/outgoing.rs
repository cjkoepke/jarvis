use crate::git;
use crate::rules::ChangedFile;
use anyhow::Result;
use std::collections::HashSet;

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

pub fn changed_files(pushed: &PushedRef, remote: Option<&str>) -> Result<Vec<ChangedFile>> {
    let Some(oldest_new_commit) = oldest_commit_the_remote_lacks(pushed, remote)? else {
        return Ok(Vec::new());
    };
    let base = parent_or_empty_tree(&oldest_new_commit)?;

    let changed_paths = git::run(&[
        "diff",
        "--name-only",
        "--diff-filter=ACMR",
        "--no-renames",
        &base,
        &pushed.local_sha,
    ])?;
    changed_paths
        .lines()
        .map(|path| changed_file(path, &base, &pushed.local_sha))
        .collect()
}

fn oldest_commit_the_remote_lacks(pushed: &PushedRef, remote: Option<&str>) -> Result<Option<String>> {
    let on_remote_branches = match remote {
        Some(remote) => format!("--remotes={remote}"),
        None => "--remotes".to_string(),
    };
    let mut rev_list = vec!["rev-list", &pushed.local_sha, "--not", &on_remote_branches];
    if remote_commit_is_known_locally(&pushed.remote_sha) {
        rev_list.push(&pushed.remote_sha);
    }
    let newest_first = git::run(&rev_list)?;
    Ok(newest_first.lines().last().map(str::to_string))
}

fn remote_commit_is_known_locally(remote_sha: &str) -> bool {
    !is_no_commit(remote_sha) && git::run(&["cat-file", "-e", remote_sha]).is_ok()
}

fn parent_or_empty_tree(commit: &str) -> Result<String> {
    let parent = git::run(&["rev-parse", "--verify", "--quiet", &format!("{commit}^")]);
    parent.or_else(|_root_commit| git::run(&["hash-object", "-t", "tree", "/dev/null"]))
}

fn changed_file(path: &str, base: &str, commit: &str) -> Result<ChangedFile> {
    let bytes = git::run_bytes(&["show", &format!("{commit}:{path}")])?;
    let zero_context_diff = git::run(&["diff", "--unified=0", "--no-color", "--no-ext-diff", base, commit, "--", path])?;
    Ok(ChangedFile {
        path: path.to_string(),
        contents: text_unless_binary(bytes),
        added_lines: added_lines(&zero_context_diff),
    })
}

fn text_unless_binary(bytes: Vec<u8>) -> Option<String> {
    String::from_utf8(bytes).ok().filter(|text| !text.contains('\0'))
}

/// @agent Reads the new side of each `@@ -a,b +c,d @@` hunk header. A missing
/// count means one line; a count of 0 is a pure deletion.
fn added_lines(zero_context_diff: &str) -> HashSet<usize> {
    let mut lines = HashSet::new();
    for hunk_header in zero_context_diff.lines().filter(|line| line.starts_with("@@")) {
        let Some(new_side) = hunk_header.split_whitespace().find(|part| part.starts_with('+')) else {
            continue;
        };
        let mut numbers = new_side[1..].split(',');
        let start: usize = numbers.next().and_then(|number| number.parse().ok()).unwrap_or(0);
        let count: usize = numbers.next().and_then(|number| number.parse().ok()).unwrap_or(1);
        lines.extend(start..start + count);
    }
    lines
}

fn is_no_commit(sha: &str) -> bool {
    sha.bytes().all(|byte| byte == b'0')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn added_lines_come_from_the_new_side_of_each_hunk() {
        let diff = "@@ -1,0 +2,2 @@\n+a\n+b\n@@ -9 +11 @@\n-x\n+y\n@@ -20,3 +21,0 @@\n-gone";
        assert_eq!(added_lines(diff), HashSet::from([2, 3, 11]));
    }
}
