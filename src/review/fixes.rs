use crate::git::{
    self,
    outgoing::{Outgoing, PushedRef},
};
use crate::settings::ReviewSettings;
use anyhow::{Context, Result};
use serde::Serialize;

const HEAD: &str = "HEAD";

/// Whether the reviewer may fix a push, and otherwise the first thing that rules it out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fixing {
    Allowed,
    SwitchedOff,
    SeveralRefs,
    RefNotCheckedOut,
    UncommittedEdits,
    ReadOnlyAgent,
}

impl Fixing {
    /// Fix commits are made on HEAD, so they are only part of the next push when the
    /// one pushed ref moves with HEAD and no uncommitted edit could end up in them.
    pub fn decide(settings: &ReviewSettings, pushed: &[&PushedRef], agent_may_write: bool) -> Result<Fixing> {
        if !settings.fix {
            return Ok(Fixing::SwitchedOff);
        }
        let [pushed] = pushed else {
            return Ok(Fixing::SeveralRefs);
        };
        if !moves_with_head(pushed) {
            return Ok(Fixing::RefNotCheckedOut);
        }
        let uncommitted_edits = git::run(&["status", "--porcelain", "--untracked-files=no"])?;
        if !uncommitted_edits.is_empty() {
            return Ok(Fixing::UncommittedEdits);
        }
        if !agent_may_write {
            return Ok(Fixing::ReadOnlyAgent);
        }
        Ok(Fixing::Allowed)
    }

    pub fn is_allowed(self) -> bool {
        self == Fixing::Allowed
    }
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
