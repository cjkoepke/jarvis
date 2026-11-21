use crate::agents::{self, hooks, skills, Agent, Approval};
use crate::ast_grep::{self, Action, PackageManager};
use crate::cli::InstallArgs;
use crate::git::hook_config;
use crate::paths::Env;
use crate::record::InstallRecord;
use crate::standards::source::Source;
use crate::standards::{self, custom, Standards};
use crate::ui::{message, Tone};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn run(env: &Env, args: &InstallArgs) -> Result<ExitCode> {
    let previous = InstallRecord::load(env);
    let source = standards_source(env, args, previous.as_ref())?;
    let agents = agents::chosen(agent_ids(args, previous.as_ref()))?;

    let standards = replace_installed_standards(env, &source)?;
    let own = match custom::pieces(&env.custom_standards_dir())?.len() {
        0 => String::new(),
        count => format!(" with {count} of your own"),
    };
    message::done(
        "standards",
        &format!(
            "{source}{own}, copied to {} ({} skills)",
            env.installed_standards_dir().display(),
            standards.skills.len()
        ),
    );

    for agent in &agents {
        install_for_agent(*agent, env, &standards)?;
    }
    remove_unselected_agents(env, &agents, previous.as_ref())?;

    hook_config::register(env)?;
    message::done("git", &format!("pre-push review in {}", env.global_gitconfig().display()));

    let agent_ids = agents.iter().map(|agent| agent.id().to_string()).collect();
    let mut record = InstallRecord {
        standards_source: source.folder().map(Path::to_path_buf),
        agent_ids,
        ast_grep_installed_by: previous.map(|record| record.ast_grep_installed_by).unwrap_or_default(),
    };
    record.save(env)?;
    if let Some(manager) = provide_ast_grep(env) {
        if !record.ast_grep_installed_by.contains(&manager) {
            record.ast_grep_installed_by.push(manager);
        }
        record.save(env)?;
    }

    if ast_grep::on_path().is_some() {
        message::summary(Tone::Good, "Installed. Run `jarvis status` to check it.");
    } else {
        message::summary(
            Tone::Caution,
            "Installed, except ast-grep, which the coding skill searches code with. Run `jarvis status` to check it.",
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn standards_source(env: &Env, args: &InstallArgs, previous: Option<&InstallRecord>) -> Result<Source> {
    let source = match &args.standards {
        Some(named) => Source::Folder(named.clone()),
        None if args.packaged => Source::Packaged,
        None => {
            let gone = previous.and_then(|record| record.standards_source.as_deref().filter(|folder| !folder.is_dir()));
            if let Some(gone) = gone {
                message::note(&format!("{} is gone, so the packaged standards are used instead", gone.display()));
            }
            InstallRecord::standards_in_effect(previous)
        }
    };
    let jarvis_dir = env.jarvis_dir().canonicalize().unwrap_or_else(|_| env.jarvis_dir());
    let source = match source {
        Source::Folder(folder) => Source::Folder(
            folder
                .canonicalize()
                .with_context(|| format!("the standards folder {} does not exist", folder.display()))?,
        ),
        Source::Packaged => Source::Packaged,
    };
    if let Some(inside) = source.folder().filter(|folder| folder.starts_with(&jarvis_dir)) {
        bail!(
            "{} is inside {}, where jarvis writes what it installs; keep your standards in another folder, put single pieces in {}, or pass `--packaged`",
            inside.display(),
            env.jarvis_dir().display(),
            env.custom_standards_dir().display()
        );
    }
    Ok(source)
}

fn agent_ids<'a>(args: &'a InstallArgs, previous: Option<&'a InstallRecord>) -> &'a [String] {
    match previous {
        Some(record) if args.agents.is_empty() => &record.agent_ids,
        _ => &args.agents,
    }
}

fn replace_installed_standards(env: &Env, source: &Source) -> Result<Standards> {
    let installed = env.installed_standards_dir();
    let assembled = env.next_standards_dir();
    if assembled.exists() {
        fs::remove_dir_all(&assembled)?;
    }
    let usable = standards::assemble(source, &env.custom_standards_dir(), &assembled).and_then(|()| Standards::load(&assembled));
    if let Err(error) = usable {
        fs::remove_dir_all(&assembled).ok();
        return Err(error.context(format!(
            "{source} with your own from {} are not usable",
            env.custom_standards_dir().display()
        )));
    }

    if installed.exists() {
        fs::remove_dir_all(&installed)?;
    }
    fs::rename(&assembled, &installed).with_context(|| format!("writing {}", installed.display()))?;
    Standards::load(&installed)
}

fn install_for_agent(agent: &dyn Agent, env: &Env, standards: &Standards) -> Result<()> {
    hooks::file::register(agent, env)?;
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

/// The package manager that put ast-grep on PATH in this run. An ast-grep that was there
/// before is never claimed, on PATH or off it, and neither is one that did not reach PATH.
fn provide_ast_grep(env: &Env) -> Option<PackageManager> {
    if let Some(found) = ast_grep::on_path() {
        message::done("ast-grep", &found.display().to_string());
        return None;
    }
    let Some(manager) = PackageManager::first_on_path() else {
        message::caution(
            "ast-grep",
            "not on PATH, and neither Homebrew nor cargo is there to install it; see https://ast-grep.github.io/guide/quick-start.html",
        );
        return None;
    };
    if manager.holds_ast_grep() {
        message::caution(
            "ast-grep",
            &format!("not on PATH, though {manager} has a copy; jarvis leaves that one as it is, so put it on PATH yourself"),
        );
        return None;
    }

    message::caution(
        "ast-grep",
        &format!("not on PATH; running `{}`", manager.command_line(Action::Install)),
    );
    if let Err(error) = manager.run(Action::Install, &env.home) {
        message::note(&format!("{error:#}; the next `jarvis install` tries again"));
        return None;
    }
    match ast_grep::on_path() {
        Some(installed) => {
            message::done("ast-grep", &format!("installed with {manager} at {}", installed.display()));
            Some(manager)
        }
        None => {
            message::note(&format!(
                "{manager} finished, but `ast-grep` is still not on PATH; add the folder {manager} installs programs into"
            ));
            None
        }
    }
}

fn remove_unselected_agents(env: &Env, selected: &[&dyn Agent], previous: Option<&InstallRecord>) -> Result<()> {
    let is_selected = |agent: &dyn Agent| selected.iter().any(|kept| kept.id() == agent.id());
    for agent in agents::all().into_iter().filter(|agent| !is_selected(*agent)) {
        hooks::file::unregister(agent, env)?;
        skills::remove(agent, env)?;
        let was_installed = previous.is_some_and(|record| record.agent_ids.iter().any(|id| id == agent.id()));
        if was_installed {
            message::done(agent.id(), "removed, no longer selected");
        }
    }
    Ok(())
}
