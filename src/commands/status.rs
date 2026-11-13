use crate::agents;
use crate::check::{self, Check, CheckSubject};
use crate::cli::Format;
use crate::files::same_contents;
use crate::git::hook_config::{self, Registration, MINIMUM_GIT_VERSION};
use crate::paths::Env;
use crate::record::InstallRecord;
use crate::review::reviewer;
use crate::standards::{Standards, CORE_MAX_CHARS};
use crate::ui::{message, Tone};
use anyhow::Result;
use std::process::ExitCode;

const STANDARDS: CheckSubject = CheckSubject {
    area: "Standards",
    name: "installed",
};
const GIT_VERSION: CheckSubject = CheckSubject {
    area: "Git",
    name: "version",
};
const GIT_REVIEW: CheckSubject = CheckSubject {
    area: "Git",
    name: "pre-push review",
};
const GIT_REVIEWER: CheckSubject = CheckSubject {
    area: "Git",
    name: "reviewer",
};

pub fn run(env: &Env, format: Format) -> Result<ExitCode> {
    let checks = gather(env);
    match format {
        Format::Table => {
            anstream::println!("{}", check::table::render(&checks));
            print_summary(&checks);
        }
        Format::Json => println!("{}", check::json::render(&checks)?),
    }
    Ok(if check::is_healthy(&checks) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn gather(env: &Env) -> Vec<Check> {
    let mut checks = Vec::new();
    match (InstallRecord::load(env), Standards::load(&env.installed_standards_dir())) {
        (Some(record), Ok(standards)) => {
            checks.push(standards_check(env, &record, &standards));
            for agent in record.agent_ids.iter().filter_map(|id| agents::by_id(id)) {
                checks.extend(agents::checks::run(agent, env, &standards));
            }
        }
        (Some(_), Err(error)) => checks.push(STANDARDS.fail(format!("{error:#}; run `jarvis install`"))),
        (None, _) => checks.push(STANDARDS.fail("jarvis is not installed here; run `jarvis install --standards <folder>`")),
    }
    checks.push(git_version_check());
    checks.push(git_review_check(env));
    checks.push(git_reviewer_check(env));
    checks
}

fn print_summary(checks: &[Check]) {
    let tally = check::tally(checks);
    if tally.failed > 0 {
        let text = format!(
            "✗ {} failed, {} passed. Each failed row says how to fix it.",
            tally.failed, tally.passed
        );
        message::summary(Tone::Bad, &text);
    } else if tally.warnings > 0 {
        message::summary(Tone::Caution, &format!("! {} passed, {} to look at.", tally.passed, tally.warnings));
    } else {
        message::summary(Tone::Good, &format!("✓ All {} checks passed.", tally.passed));
    }
}

fn standards_check(env: &Env, record: &InstallRecord, standards: &Standards) -> Check {
    let contents = format!(
        "core {} of {CORE_MAX_CHARS} characters · {} skills",
        standards.core.chars().count(),
        standards.skills.len()
    );
    let source = &record.standards_source;
    if !source.is_dir() {
        return STANDARDS.warn(format!(
            "{contents}. The source {} is gone, so this copy cannot be compared with it",
            source.display()
        ));
    }
    if !same_contents(source, &env.installed_standards_dir(), None) {
        return STANDARDS.warn(format!(
            "out of date: {} changed since the last install; run `jarvis install`",
            source.display()
        ));
    }
    STANDARDS.ok(contents)
}

fn git_version_check() -> Check {
    let (minimum_major, minimum_minor) = MINIMUM_GIT_VERSION;
    match hook_config::installed_git_version() {
        Some(version) if version >= MINIMUM_GIT_VERSION => GIT_VERSION.ok(format!("{}.{}", version.0, version.1)),
        Some((major, minor)) => GIT_VERSION.fail(format!(
            "{major}.{minor} is older than {minimum_major}.{minimum_minor}, so the pre-push review will not run; upgrade git"
        )),
        None => GIT_VERSION.fail("git is not on PATH; install git"),
    }
}

fn git_review_check(env: &Env) -> Check {
    let gitconfig = env.global_gitconfig();
    match hook_config::registration(env) {
        Registration::Active { executable } if executable.is_file() => GIT_REVIEW.ok(format!("registered in {}", gitconfig.display())),
        Registration::Active { executable } => GIT_REVIEW.fail(format!(
            "registered to run {}, which no longer exists; run `jarvis install`",
            executable.display()
        )),
        Registration::Missing => GIT_REVIEW.fail(format!("not registered in {}; run `jarvis install`", gitconfig.display())),
        Registration::WrongEvent => GIT_REVIEW.fail(format!(
            "registered in {} for the wrong git event; run `jarvis install`",
            gitconfig.display()
        )),
        Registration::SwitchedOff => GIT_REVIEW.warn(format!("switched off by `hook.jarvis.enabled` in {}", gitconfig.display())),
    }
}

fn git_reviewer_check(env: &Env) -> Check {
    match reviewer::choose(env) {
        Some(agent) => GIT_REVIEWER.ok(agent.display_name()),
        None => GIT_REVIEWER.warn("no installed agent is on PATH, so pushes go out unreviewed"),
    }
}
