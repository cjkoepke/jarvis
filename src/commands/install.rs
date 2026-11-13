use crate::agents::{self, hooks_file, skills, Agent};
use crate::cli::InstallArgs;
use crate::files::copy_dir;
use crate::git::hook_config;
use crate::paths::Env;
use crate::standards::Standards;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn run(env: &Env, args: &InstallArgs) -> Result<ExitCode> {
    let agents = agents::chosen(&args.agents)?;

    let standards = replace_installed_standards(env, &args.standards)?;
    println!(
        "standards  copied to {} ({} skills, {} rules)",
        env.installed_standards_dir().display(),
        standards.skills.len(),
        standards.rules.len()
    );

    for agent in &agents {
        install_for_agent(*agent, env, &standards)?;
    }
    agents::record_installed(env, &agents)?;

    hook_config::register(env)?;
    println!("{:<10} pre-push review in {}", "git", env.global_gitconfig().display());

    println!("\nInstalled. Run `jarvis doctor` to check it.");
    Ok(ExitCode::SUCCESS)
}

/// @agent The source is loaded before anything is copied, so a broken standards
/// folder can never replace a working install.
fn replace_installed_standards(env: &Env, source: &Path) -> Result<Standards> {
    Standards::load(source).with_context(|| format!("the standards in {} are not usable", source.display()))?;

    let source = source.canonicalize()?;
    let installed = env.installed_standards_dir();
    let source_is_the_installed_copy = installed.canonicalize().ok().as_ref() == Some(&source);
    if !source_is_the_installed_copy {
        if installed.exists() {
            fs::remove_dir_all(&installed)?;
        }
        copy_dir(&source, &installed)?;
    }
    Standards::load(&installed)
}

fn install_for_agent(agent: &dyn Agent, env: &Env, standards: &Standards) -> Result<()> {
    hooks_file::register(agent, env)?;
    println!("{:<10} hooks in {}", agent.id(), agent.hooks_file(env).display());

    let skipped_skills = skills::install(agent, env, standards)?;
    for note in skipped_skills.iter().map(String::as_str).chain(agent.manual_step_after_install()) {
        println!("{:<10} note: {note}", "");
    }
    Ok(())
}
