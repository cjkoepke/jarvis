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
    if env.jarvis_dir().exists() {
        fs::remove_dir_all(env.jarvis_dir())?;
    }

    message::done("jarvis", &format!("removed from {}", env.home.display()));
    Ok(ExitCode::SUCCESS)
}
