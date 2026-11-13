use crate::git::outgoing::{self, Outgoing, PushedRef};
use crate::paths::Env;
use crate::review::verdict::{self, Verdict};
use crate::review::{prompt, reviewer};
use crate::standards::Standards;
use crate::ui::{message, Tone};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::io::Read;
use std::process::ExitCode;

/// @agent Any internal error lets the push through. The review exists to improve
/// what goes out, so it must never be the reason a push cannot happen. Output goes
/// to stderr because git relays a hook's stderr to whoever ran `git push`, which
/// is how an agent receives the result.
pub fn run(env: &Env, git_args: &[String]) -> ExitCode {
    review(env, git_args).unwrap_or_else(|error| {
        message::review_verdict(Tone::Caution, &format!("review skipped ({error:#}). The push continues."));
        ExitCode::SUCCESS
    })
}

fn review(env: &Env, git_args: &[String]) -> Result<ExitCode> {
    let outgoing = outgoing_commits(git_args.first().map(String::as_str))?;
    if outgoing.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    let standards = Standards::load(&env.installed_standards_dir())?;
    let reviewer = reviewer::choose(env).context("no installed agent is on PATH")?;
    message::review_verdict(
        Tone::Muted,
        &format!(
            "{} is reviewing the outgoing commits. This can take a few minutes.",
            reviewer.display_name()
        ),
    );

    let reply = reviewer::ask(reviewer, env, &prompt::build(&standards, &outgoing))?;
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

fn outgoing_commits(remote: Option<&str>) -> Result<Vec<Outgoing>> {
    let mut refs_being_pushed = String::new();
    std::io::stdin().read_to_string(&mut refs_being_pushed)?;

    let mut outgoing = Vec::new();
    let mut seen_commits = HashSet::new();
    for pushed in refs_being_pushed.lines().filter_map(PushedRef::parse) {
        let already_seen = !seen_commits.insert(pushed.local_sha.clone());
        if pushed.deletes_the_ref() || already_seen {
            continue;
        }
        outgoing.extend(outgoing::commits_the_remote_lacks(&pushed, remote)?);
    }
    Ok(outgoing)
}
