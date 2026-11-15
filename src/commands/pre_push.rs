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

struct Push {
    remote: String,
    remote_url: String,
    refs: Vec<PushedRef>,
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

    let refused_now = refs_to_refuse(env, &push, &refused).unwrap_or_else(|error| {
        review_skipped(&error);
        Vec::new()
    });
    if refused_now.is_empty() {
        push.refs.iter().for_each(|pushed| refused.forget(&push.remote_url, pushed));
    } else {
        refused_now.iter().for_each(|pushed| refused.remember(&push.remote_url, pushed));
    }
    if refused != recorded {
        if let Err(error) = refused.save(env) {
            message::review_verdict(Tone::Caution, &format!("{error:#}"));
        }
    }

    if refused_now.is_empty() {
        return ExitCode::SUCCESS;
    }
    let next_step = "push refused. Fix the findings above, commit, and push again. That push goes out without another review.";
    message::review_verdict(Tone::Bad, next_step);
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

fn refs_to_refuse<'a>(env: &Env, push: &'a Push, refused: &RefusedPushes) -> Result<Vec<&'a PushedRef>> {
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
        return Ok(Vec::new());
    }
    Ok(match ask_reviewer(env, &push.remote, &unreviewed)? {
        Verdict::Pass => Vec::new(),
        Verdict::Fail => unreviewed,
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
