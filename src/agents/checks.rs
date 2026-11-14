use super::{hooks_file, skills, Agent, Approval};
use crate::check::{Check, CheckSubject};
use crate::files::find_on_path;
use crate::paths::Env;
use crate::standards::Standards;

pub fn run(agent: &dyn Agent, env: &Env, standards: &Standards) -> Vec<Check> {
    let mut checks = vec![
        executable_check(agent),
        hooks_check(agent, env),
        skills_check(agent, env, standards),
    ];
    checks.extend(approval_check(agent, env));
    checks
}

fn subject<'a>(agent: &'a dyn Agent, name: &'a str) -> CheckSubject<'a> {
    CheckSubject {
        area: agent.display_name(),
        name,
    }
}

fn executable_check(agent: &dyn Agent) -> Check {
    let binary = subject(agent, "binary");
    match find_on_path(agent.executable_name()) {
        Some(path) => binary.ok(path.display().to_string()),
        None => binary.warn(format!("`{}` is not on PATH", agent.executable_name())),
    }
}

fn hooks_check(agent: &dyn Agent, env: &Env) -> Check {
    let hooks = subject(agent, "hooks");
    let registered = hooks_file::registered(agent, env);

    let is_registered = |event: &str| registered.iter().any(|hook| hook.event == event);
    let unregistered: Vec<&str> = agent
        .context_events()
        .iter()
        .copied()
        .filter(|event| !is_registered(event))
        .collect();
    if !unregistered.is_empty() {
        return hooks.fail(format!(
            "{} not registered in {}; run `jarvis install`",
            unregistered.join(", "),
            agent.hooks_file(env).display()
        ));
    }

    let missing_executable = registered
        .iter()
        .map(|hook| &hook.executable)
        .find(|executable| !executable.is_file());
    if let Some(executable) = missing_executable {
        return hooks.fail(format!(
            "registered to run {}, which no longer exists; run `jarvis install`",
            executable.display()
        ));
    }
    hooks.ok(agent.context_events().join(", "))
}

fn skills_check(agent: &dyn Agent, env: &Env, standards: &Standards) -> Check {
    let skills = subject(agent, "skills");
    let skills_dir = agent.skills_dir(env);

    let not_installed = skills::not_installed(agent, env, standards);
    if !not_installed.is_empty() {
        return skills.fail(format!(
            "{} not installed in {}; run `jarvis install`",
            not_installed.join(", "),
            skills_dir.display()
        ));
    }
    let out_of_date = skills::out_of_date(agent, env, standards);
    if !out_of_date.is_empty() {
        return skills.fail(format!(
            "{} in {} differ from the installed standards; run `jarvis install`",
            out_of_date.join(", "),
            skills_dir.display()
        ));
    }
    skills.ok(format!("{} installed", standards.skills.len()))
}

fn approval_check(agent: &dyn Agent, env: &Env) -> Option<Check> {
    let approval = subject(agent, "approval");
    match agent.approval(env) {
        Approval::NotRequired => None,
        Approval::Granted => Some(approval.ok("approved")),
        Approval::Pending { next_step } => Some(approval.warn(next_step)),
    }
}
