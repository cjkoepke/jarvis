use crate::agents;
use crate::check::{self, Check, CheckSubject};
use crate::cli::Format;
use crate::files::same_contents;
use crate::git::hook_config::{self, Registration, MINIMUM_GIT_VERSION};
use crate::paths::Env;
use crate::record::InstallRecord;
use crate::review::reviewer;
use crate::settings::Settings;
use crate::standards::source::Source;
use crate::standards::{self, custom, Standards, CORE_MAX_CHARS};
use anyhow::Result;
use std::process::ExitCode;

const STANDARDS: CheckSubject = CheckSubject {
    area: "Standards",
    name: "installed",
};
const CUSTOM: CheckSubject = CheckSubject {
    area: "Standards",
    name: "your own",
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
    check::report::print(&gather(env), format)
}

fn gather(env: &Env) -> Vec<Check> {
    let mut checks = Vec::new();
    let standards = Standards::load(&env.installed_standards_dir());
    match (InstallRecord::load(env), &standards) {
        (Some(record), Ok(standards)) => {
            checks.push(standards_check(env, &record, standards));
            checks.push(custom_check(env));
            for agent in record.agent_ids.iter().filter_map(|id| agents::by_id(id)) {
                checks.extend(agents::checks::run(agent, env, standards));
            }
        }
        (Some(_), Err(error)) => checks.push(STANDARDS.fail(format!("{error:#}; run `jarvis install`"))),
        (None, _) => checks.push(STANDARDS.fail("jarvis is not installed here; run `jarvis install`")),
    }
    checks.push(git_version_check());
    checks.push(git_review_check(env));
    if let Ok(standards) = &standards {
        checks.push(git_reviewer_check(env, standards));
    }
    checks
}

fn standards_check(env: &Env, record: &InstallRecord, standards: &Standards) -> Check {
    let contents = format!(
        "core {} of {CORE_MAX_CHARS} characters · {} skills",
        standards.core.chars().count(),
        standards.skills.len()
    );
    let source = record.standards();
    if let Some(gone) = source.folder().filter(|folder| !folder.is_dir()) {
        return STANDARDS.warn(format!(
            "{contents}. The source {} is gone; run `jarvis install` to use the packaged standards",
            gone.display()
        ));
    }
    match is_up_to_date(env, &source) {
        Ok(true) => STANDARDS.ok(contents),
        Ok(false) => STANDARDS.warn(format!(
            "out of date: {source} or your own in {} changed since the last install; run `jarvis install`",
            env.custom_standards_dir().display()
        )),
        Err(error) => STANDARDS.warn(format!("{contents}. This copy cannot be compared with its source: {error:#}")),
    }
}

fn is_up_to_date(env: &Env, source: &Source) -> Result<bool> {
    let delivered_now = tempfile::tempdir()?;
    standards::assemble(source, &env.custom_standards_dir(), delivered_now.path())?;
    Ok(same_contents(delivered_now.path(), &env.installed_standards_dir(), None))
}

fn custom_check(env: &Env) -> Check {
    let custom_dir = env.custom_standards_dir();
    let (pieces, unusable) = match custom::pieces(&custom_dir).and_then(|pieces| Ok((pieces, custom::unusable(&custom_dir)?))) {
        Ok(found) => found,
        Err(error) => return CUSTOM.fail(format!("{error:#}; repair that folder")),
    };
    if !unusable.is_empty() {
        return CUSTOM.warn(format!(
            "{} in {} are not delivered: a core file ends in `.md` and a skill is a folder holding `SKILL.md`",
            unusable.join(", "),
            custom_dir.display()
        ));
    }
    if pieces.is_empty() {
        return CUSTOM.ok("none; `jarvis clone` copies a core file or a skill to change");
    }
    CUSTOM.ok(format!("{} in {}", pieces.join(", "), custom_dir.display()))
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

fn git_reviewer_check(env: &Env, standards: &Standards) -> Check {
    let unreviewed = |error: anyhow::Error| GIT_REVIEWER.warn(format!("{error:#}, so pushes go out unreviewed"));
    let settings = match Settings::load(&env.settings_file()) {
        Ok(settings) => settings,
        Err(error) => return unreviewed(error),
    };
    match reviewer::choose(standards, &settings.review) {
        Ok(reviewer) => GIT_REVIEWER.ok(reviewer.to_string()),
        Err(error) => unreviewed(error),
    }
}
