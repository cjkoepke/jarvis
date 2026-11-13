use crate::agents::{self, hooks_file, skills, Agent, Approval};
use crate::cli::InstallArgs;
use crate::files::copy_dir;
use crate::git::hook_config;
use crate::paths::Env;
use crate::record::InstallRecord;
use crate::standards::Standards;
use crate::ui::{message, Tone};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_STANDARDS: &str = "standards";

/// @agent Flags left out fall back to the previous install's values, so a bare
/// `jarvis install` repairs or refreshes an install from any directory.
pub fn run(env: &Env, args: &InstallArgs) -> Result<ExitCode> {
    let previous = InstallRecord::load(env);
    let source = standards_source(args, previous.as_ref())?;
    let agents = agents::chosen(agent_ids(args, previous.as_ref()))?;

    let standards = replace_installed_standards(env, &source)?;
    message::done(
        "standards",
        &format!(
            "copied to {} ({} skills, {} rules)",
            env.installed_standards_dir().display(),
            standards.skills.len(),
            standards.rules.len()
        ),
    );

    for agent in &agents {
        install_for_agent(*agent, env, &standards)?;
    }
    remove_unselected_agents(env, &agents, previous.as_ref())?;

    hook_config::register(env)?;
    message::done("git", &format!("pre-push review in {}", env.global_gitconfig().display()));

    let agent_ids = agents.iter().map(|agent| agent.id().to_string()).collect();
    InstallRecord {
        standards_source: source,
        agent_ids,
    }
    .save(env)?;

    message::summary(Tone::Good, "Installed. Run `jarvis status` to check it.");
    Ok(ExitCode::SUCCESS)
}

fn standards_source(args: &InstallArgs, previous: Option<&InstallRecord>) -> Result<PathBuf> {
    let previous_source = previous.map(|record| record.standards_source.clone());
    let source = args
        .standards
        .clone()
        .or(previous_source)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STANDARDS));
    source
        .canonicalize()
        .with_context(|| format!("the standards folder {} does not exist", source.display()))
}

fn agent_ids<'a>(args: &'a InstallArgs, previous: Option<&'a InstallRecord>) -> &'a [String] {
    match previous {
        Some(record) if args.agents.is_empty() => &record.agent_ids,
        _ => &args.agents,
    }
}

/// @agent The source is loaded before anything is copied, so a broken standards
/// folder can never replace a working install.
fn replace_installed_standards(env: &Env, source: &Path) -> Result<Standards> {
    Standards::load(source).with_context(|| format!("the standards in {} are not usable", source.display()))?;

    let installed = env.installed_standards_dir();
    let source_is_the_installed_copy = installed.canonicalize().ok().as_deref() == Some(source);
    if !source_is_the_installed_copy {
        if installed.exists() {
            fs::remove_dir_all(&installed)?;
        }
        copy_dir(source, &installed)?;
    }
    Standards::load(&installed)
}

fn install_for_agent(agent: &dyn Agent, env: &Env, standards: &Standards) -> Result<()> {
    hooks_file::register(agent, env)?;
    message::done(agent.id(), &format!("hooks in {}", agent.hooks_file(env).display()));

    let skipped_skills = skills::install(agent, env, standards)?;
    for skipped in &skipped_skills {
        message::note(skipped);
    }
    if let Approval::Pending { next_step } = agent.approval(env) {
        message::note(next_step);
    }
    Ok(())
}

/// @agent Without this, narrowing `--agents` would leave the dropped agent's hooks
/// and skills active while `status` stopped reporting on them.
fn remove_unselected_agents(env: &Env, selected: &[&dyn Agent], previous: Option<&InstallRecord>) -> Result<()> {
    let is_selected = |agent: &dyn Agent| selected.iter().any(|kept| kept.id() == agent.id());
    for agent in agents::all().into_iter().filter(|agent| !is_selected(*agent)) {
        hooks_file::unregister(agent, env)?;
        skills::remove(agent, env)?;
        let was_installed = previous.is_some_and(|record| record.agent_ids.iter().any(|id| id == agent.id()));
        if was_installed {
            message::done(agent.id(), "removed, no longer selected");
        }
    }
    Ok(())
}
