use crate::git::{
    self,
    outgoing::{Outgoing, PushedRef},
};
use anyhow::{Context, Result};

const HEAD: &str = "HEAD";

/// Whether fix commits made on HEAD would be part of the next push of these refs.
pub fn can_follow(pushed: &[&PushedRef]) -> Result<bool> {
    let [pushed] = pushed else {
        return Ok(false);
    };
    let uncommitted_edits = git::run(&["status", "--porcelain", "--untracked-files=no"])?;
    Ok(moves_with_head(pushed) && uncommitted_edits.is_empty())
}

fn moves_with_head(pushed: &PushedRef) -> bool {
    let checked_out_branch = git::run(&["symbolic-ref", "--quiet", HEAD]);
    pushed.local_ref == HEAD || checked_out_branch.is_ok_and(|branch| branch == pushed.local_ref)
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
