use crate::git::outgoing::{Outgoing, PushedRef};
use crate::paths::Env;
use crate::review::refused::RefusedPushes;
use crate::review::verdict::{self, Verdict};
use crate::review::{fixes, reviewer};
use crate::settings::Settings;
use crate::standards::Standards;
use crate::ui::{message, Tone};
use anyhow::{Context, Result};
use std::io::Read;
use std::process::ExitCode;

const PUSHER_FIXES: &str = "Fix the findings above, commit, and push again.";
const REVIEWER_FIXED_NOTHING: &str =
    "The reviewer reported fixes but committed none on top of the pushed commits. Check the working tree, commit, and push again.";

struct Push {
    remote: String,
    remote_url: String,
    refs: Vec<PushedRef>,
}

struct Refusal<'a> {
    refs: Vec<&'a PushedRef>,
    next_step: String,
}

pub fn run(env: &Env, git_args: &[String]) -> ExitCode {
    let push = match Push::read(git_args) {
        Ok(push) => push,
        Err(error) => {
            review_skipped(&error);
            return ExitCode::SUCCESS;
        }
    };
    let mut refused = RefusedPushes::load(env);
    let recorded = refused.clone();

    let refusal = review(env, &push, &refused).unwrap_or_else(|error| {
        review_skipped(&error);
        None
    });
    match &refusal {
        Some(refusal) => refusal.refs.iter().for_each(|pushed| refused.remember(&push.remote_url, pushed)),
        None => push.refs.iter().for_each(|pushed| refused.forget(&push.remote_url, pushed)),
    }
    if refused != recorded {
        if let Err(error) = refused.save(env) {
            message::review_verdict(Tone::Caution, &format!("{error:#}"));
        }
    }

    let Some(refusal) = refusal else {
        return ExitCode::SUCCESS;
    };
    let next_step = format!("push refused. {} That push goes out without another review.", refusal.next_step);
    message::review_verdict(Tone::Bad, &next_step);
    ExitCode::FAILURE
}

impl Push {
    fn read(git_args: &[String]) -> Result<Push> {
        let remote = git_args.first().context("git named no remote")?;
        let mut refs_being_pushed = String::new();
        std::io::stdin().read_to_string(&mut refs_being_pushed)?;
        Ok(Push {
            remote: remote.clone(),
            remote_url: git_args.get(1).unwrap_or(remote).clone(),
            refs: refs_being_pushed.lines().filter_map(PushedRef::parse).collect(),
        })
    }
}

fn review_skipped(error: &anyhow::Error) {
    message::review_verdict(Tone::Caution, &format!("review skipped ({error:#}). The push continues."));
}

fn review<'a>(env: &Env, push: &'a Push, refused: &RefusedPushes) -> Result<Option<Refusal<'a>>> {
    let mut unreviewed = Vec::new();
    let mut some_were_refused_before = false;
    for pushed in &push.refs {
        if pushed.deletes_the_ref() || !pushed.has_commits_the_remote_lacks(&push.remote)? {
            continue;
        }
        if refused.holds(&push.remote_url, pushed) {
            some_were_refused_before = true;
        } else {
            unreviewed.push(pushed);
        }
    }

    if unreviewed.is_empty() {
        if some_were_refused_before {
            message::review_verdict(Tone::Muted, "already reviewed once, so this push goes out without another review.");
        }
        return Ok(None);
    }

    let mut commits: Vec<String> = Vec::new();
    for pushed in &unreviewed {
        if !commits.contains(&pushed.local_sha) {
            commits.push(pushed.local_sha.clone());
        }
    }
    let outgoing = Outgoing {
        remote: push.remote.clone(),
        commits,
    };
    let next_step = match ask_reviewer(env, &outgoing)? {
        Verdict::Pass => return Ok(None),
        Verdict::Fail => PUSHER_FIXES.to_string(),
        Verdict::Fixed => next_step_after_fixes(&outgoing)?,
    };
    Ok(Some(Refusal {
        refs: unreviewed,
        next_step,
    }))
}

fn next_step_after_fixes(outgoing: &Outgoing) -> Result<String> {
    Ok(match fixes::commits_on_top_of(outgoing)? {
        0 => REVIEWER_FIXED_NOTHING.to_string(),
        1 => "The reviewer fixed the findings above in 1 commit. Push again to send it.".to_string(),
        count => format!("The reviewer fixed the findings above in {count} commits. Push again to send them."),
    })
}

fn ask_reviewer(env: &Env, outgoing: &Outgoing) -> Result<Verdict> {
    let standards = Standards::load(&env.installed_standards_dir())?;
    let settings = Settings::load(&env.settings_file())?;
    let mut reviewer = reviewer::choose(&standards, &settings.review)?;
    reviewer.may_fix = settings.review.fix && fixes::can_follow(outgoing)?;
    message::review_verdict(
        Tone::Muted,
        &format!("{reviewer} is reviewing the outgoing commits. This can take a few minutes."),
    );

    let reply = reviewer::ask(&reviewer, env, outgoing)?;
    let review = verdict::parse(&reply).context("the reviewer gave no verdict")?;
    message::review_findings(&review.findings);
    if let Verdict::Pass = review.verdict {
        message::review_verdict(Tone::Good, "review passed.");
    }
    Ok(review.verdict)
}
