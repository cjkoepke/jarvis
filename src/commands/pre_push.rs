use crate::git::outgoing::{Outgoing, PushedRef};
use crate::paths::Env;
use crate::review::refused::RefusedPushes;
use crate::review::reviewer;
use crate::review::verdict::{self, Verdict};
use crate::settings::Settings;
use crate::standards::Standards;
use crate::ui::{message, Tone};
use anyhow::{Context, Result};
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
    let remote_url = git_args.get(1).unwrap_or(remote);
    let pushed = refs_with_commits_the_remote_lacks(remote)?;
    if pushed.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    let mut refused = RefusedPushes::load(env);
    let unreviewed: Vec<&PushedRef> = pushed.iter().filter(|pushed| !refused.holds(remote_url, pushed)).collect();
    let verdict = if unreviewed.is_empty() {
        message::review_verdict(Tone::Muted, "already reviewed once, so this push goes out without another review.");
        Verdict::Pass
    } else {
        ask_reviewer(env, remote, &unreviewed)?
    };

    let before = refused.clone();
    match verdict {
        Verdict::Pass => pushed.iter().for_each(|pushed| refused.forget(remote_url, pushed)),
        Verdict::Fail => unreviewed.iter().for_each(|pushed| refused.remember(remote_url, pushed)),
    }
    if refused != before {
        refused.save(env)?;
    }

    Ok(match verdict {
        Verdict::Pass => ExitCode::SUCCESS,
        Verdict::Fail => {
            let next_step = "push refused. Fix the findings above, commit, and push again. That push goes out without another review.";
            message::review_verdict(Tone::Bad, next_step);
            ExitCode::FAILURE
        }
    })
}

fn ask_reviewer(env: &Env, remote: &str, unreviewed: &[&PushedRef]) -> Result<Verdict> {
    let mut commits: Vec<String> = Vec::new();
    for pushed in unreviewed {
        if !commits.contains(&pushed.local_sha) {
            commits.push(pushed.local_sha.clone());
        }
    }
    let outgoing = Outgoing {
        remote: remote.to_string(),
        commits,
    };

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
    if let Verdict::Pass = review.verdict {
        message::review_verdict(Tone::Good, "review passed.");
    }
    Ok(review.verdict)
}

fn refs_with_commits_the_remote_lacks(remote: &str) -> Result<Vec<PushedRef>> {
    let mut refs_being_pushed = String::new();
    std::io::stdin().read_to_string(&mut refs_being_pushed)?;

    let mut pushed_refs = Vec::new();
    for pushed in refs_being_pushed.lines().filter_map(PushedRef::parse) {
        if !pushed.deletes_the_ref() && pushed.has_commits_the_remote_lacks(remote)? {
            pushed_refs.push(pushed);
        }
    }
    Ok(pushed_refs)
}
