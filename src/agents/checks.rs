use super::{hooks_file, skills, Agent};
use crate::check::Check;
use crate::files::find_on_path;
use crate::paths::Env;
use crate::standards::Standards;

pub fn run(agent: &dyn Agent, env: &Env, standards: &Standards) -> Vec<Check> {
    let mut checks = vec![
        executable_check(agent),
        hooks_check(agent, env),
        skills_check(agent, env, standards),
    ];
    if let Some(step) = agent.manual_step_after_install() {
        checks.push(Check::warn(
            format!("{} manual step", agent.display_name()),
            format!("jarvis cannot check this: {step}"),
        ));
    }
    checks
}

fn executable_check(agent: &dyn Agent) -> Check {
    let name = format!("{} binary", agent.display_name());
    match find_on_path(agent.executable_name()) {
        Some(path) => Check::ok(name, path.display().to_string()),
        None => Check::warn(name, format!("`{}` is not on PATH", agent.executable_name())),
    }
}

fn hooks_check(agent: &dyn Agent, env: &Env) -> Check {
    let name = format!("{} hooks", agent.display_name());
    let hooks_file = agent.hooks_file(env);
    let unregistered = hooks_file::unregistered_events(agent, env);
    if unregistered.is_empty() {
        Check::ok(name, format!("{} in {}", agent.context_events().join(", "), hooks_file.display()))
    } else {
        Check::fail(
            name,
            format!(
                "{} not registered in {}; run `jarvis install`",
                unregistered.join(", "),
                hooks_file.display()
            ),
        )
    }
}

fn skills_check(agent: &dyn Agent, env: &Env, standards: &Standards) -> Check {
    let name = format!("{} skills", agent.display_name());
    let skills_dir = agent.skills_dir(env);
    let not_installed = skills::not_installed(agent, env, standards);
    if not_installed.is_empty() {
        Check::ok(name, format!("{} in {}", standards.skills.len(), skills_dir.display()))
    } else {
        Check::fail(
            name,
            format!("{} not installed in {}", not_installed.join(", "), skills_dir.display()),
        )
    }
}
