use crate::agents::{self, hooks_file, skills};
use crate::git::hook_config;
use crate::paths::Env;
use crate::ui::message;
use anyhow::Result;
use std::fs;
use std::process::ExitCode;

pub fn run(env: &Env) -> Result<ExitCode> {
    for agent in agents::all() {
        hooks_file::unregister(agent, env)?;
        skills::remove(agent, env)?;
    }
    hook_config::unregister(env)?;
    remove_what_jarvis_wrote_for_itself(env)?;

    message::done("jarvis", &format!("removed from {}", env.home.display()));
    let custom_dir = env.custom_standards_dir();
    if custom_dir.exists() {
        message::note(&format!("your own standards stay in {}", custom_dir.display()));
    }
    Ok(ExitCode::SUCCESS)
}

/// A person's own standards stay, and with them the folder that holds them.
fn remove_what_jarvis_wrote_for_itself(env: &Env) -> Result<()> {
    for folder in [env.installed_standards_dir(), env.next_standards_dir(), env.reviews_dir()] {
        if folder.exists() {
            fs::remove_dir_all(folder)?;
        }
    }
    for file in [env.settings_file(), env.install_record_file(), env.refused_pushes_file()] {
        if file.exists() {
            fs::remove_file(file)?;
        }
    }
    fs::remove_dir(env.jarvis_dir()).ok();
    Ok(())
}
