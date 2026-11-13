use crate::agents;
use crate::check::{Check, Status};
use crate::git::hook_config::{self, MINIMUM_GIT_VERSION};
use crate::paths::Env;
use crate::standards::{Standards, CORE_MAX_CHARS};
use anyhow::Result;
use std::process::ExitCode;

pub fn run(env: &Env) -> Result<ExitCode> {
    let mut checks = Vec::new();

    match Standards::load(&env.installed_standards_dir()) {
        Ok(standards) => {
            checks.push(standards_check(&standards));
            for agent in agents::installed(env) {
                checks.extend(agents::checks::run(agent, env, &standards));
            }
        }
        Err(error) => checks.push(Check::fail("standards", format!("{error:#}; run `jarvis install`"))),
    }
    checks.push(git_version_check());
    checks.push(git_hook_check(env));

    for check in &checks {
        println!("{:<5} {}: {}", check.label(), check.name, check.detail);
    }
    let any_failed = checks.iter().any(|check| check.status == Status::Fail);
    Ok(if any_failed { ExitCode::FAILURE } else { ExitCode::SUCCESS })
}

fn standards_check(standards: &Standards) -> Check {
    let detail = format!(
        "core {} of {CORE_MAX_CHARS} characters, {} skills, {} rules",
        standards.core.chars().count(),
        standards.skills.len(),
        standards.rules.len()
    );
    Check::ok("standards", detail)
}

fn git_version_check() -> Check {
    let (minimum_major, minimum_minor) = MINIMUM_GIT_VERSION;
    match hook_config::installed_git_version() {
        Some(version) if version >= MINIMUM_GIT_VERSION => Check::ok("git version", format!("{}.{}", version.0, version.1)),
        Some((major, minor)) => Check::fail(
            "git version",
            format!("{major}.{minor} is older than {minimum_major}.{minimum_minor}; the pre-push review will not run"),
        ),
        None => Check::fail("git version", "git is not on PATH"),
    }
}

fn git_hook_check(env: &Env) -> Check {
    let gitconfig = env.global_gitconfig();
    if hook_config::is_registered(env) {
        Check::ok("git pre-push review", gitconfig.display().to_string())
    } else {
        Check::fail(
            "git pre-push review",
            format!("not registered in {}; run `jarvis install`", gitconfig.display()),
        )
    }
}
