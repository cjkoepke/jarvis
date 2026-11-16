use crate::git::{self, outgoing::Outgoing};
use anyhow::{Context, Result};

/// Whether fix commits made on HEAD would be part of the next push of these commits.
pub fn can_follow(outgoing: &Outgoing) -> Result<bool> {
    let head = git::run(&["rev-parse", "HEAD"])?;
    let pushes_only_head = outgoing.commits == [head];
    let uncommitted_edits = git::run(&["status", "--porcelain", "--untracked-files=no"])?;
    Ok(pushes_only_head && uncommitted_edits.is_empty())
}

/// How many commits HEAD gained on top of the pushed one. Zero when it gained none
/// or no longer contains the pushed commit.
pub fn commits_on_top_of(outgoing: &Outgoing) -> Result<usize> {
    let [pushed_tip] = &outgoing.commits[..] else {
        return Ok(0);
    };
    let counts = git::run(&["rev-list", "--left-right", "--count", &format!("{pushed_tip}...HEAD")])?;
    let (missing_from_head, added_to_head) = counts.split_once('\t').context("git counted no commits")?;
    if missing_from_head != "0" {
        return Ok(0);
    }
    Ok(added_to_head.parse()?)
}
