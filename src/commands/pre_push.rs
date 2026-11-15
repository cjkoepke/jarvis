use crate::git::outgoing::{Outgoing, PushedRef};
use crate::paths::Env;
use crate::review::reviewer;
use crate::review::verdict::{self, Verdict};
use crate::settings::Settings;
use crate::standards::Standards;
use crate::ui::{message, Tone};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::Read;
use std::process::ExitCode;

pub fn run(env: &Env, git_args: &[String]) -> ExitCode {
    review(env, git_args).unwrap_or_else(|error| {
        message::review_verdict(Tone::Caution, &format!("review skipped ({error:#}). The push continues."));
        ExitCode::SUCCESS
    })
}

fn review(env: &Env, git_args: &[String]) -> Result<ExitCode> {
    let remote = git_args.first().context("git named no remote")?;
    let outgoing = outgoing_commits(remote)?;
    if outgoing.commits.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    let standards = Standards::load(&env.installed_standards_dir())?;
    let settings = Settings::load(&env.settings_file())?;
    let reviewer = reviewer::choose(&standards, &settings.review)?;
    message::review_verdict(
        Tone::Muted,
        &format!("{reviewer} is reviewing the outgoing commits. This can take a few minutes."),
    );

    let reply = reviewer::ask(&reviewer, env, &outgoing)?;
    let review = verdict::parse(&reply).context("the reviewer gave no verdict")?;
    message::review_findings(&review.findings);
    Ok(match review.verdict {
        Verdict::Pass => {
            message::review_verdict(Tone::Good, "review passed.");
            ExitCode::SUCCESS
        }
        Verdict::Fail => {
            message::review_verdict(Tone::Bad, "push refused. Fix the findings above, commit, and push again.");
            ExitCode::FAILURE
        }
    })
}

fn outgoing_commits(remote: &str) -> Result<Outgoing> {
    let mut refs_being_pushed = String::new();
    std::io::stdin().read_to_string(&mut refs_being_pushed)?;

    let mut commits = Vec::new();
    let mut seen_commits = HashSet::new();
    for pushed in refs_being_pushed.lines().filter_map(PushedRef::parse) {
        let already_seen = !seen_commits.insert(pushed.local_sha.clone());
        if pushed.deletes_the_ref() || already_seen || !pushed.has_commits_the_remote_lacks(remote)? {
            continue;
        }
        commits.push(pushed.local_sha);
    }
    Ok(Outgoing {
        remote: remote.to_string(),
        commits,
    })
}
