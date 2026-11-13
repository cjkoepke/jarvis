use super::require_flag_for_real_home;
use crate::agents::{self, hooks_file, skills};
use crate::git::hook_config;
use crate::paths::Env;
use anyhow::Result;
use std::fs;
use std::process::ExitCode;

pub fn run(env: &Env, real_flag: bool) -> Result<ExitCode> {
    require_flag_for_real_home(env, real_flag)?;

    for agent in agents::all() {
        hooks_file::unregister(agent, env)?;
        skills::remove(agent, env)?;
    }
    hook_config::unregister(env)?;
    if env.jarvis_dir().exists() {
        fs::remove_dir_all(env.jarvis_dir())?;
    }

    println!("Removed jarvis from {}.", env.home.display());
    Ok(ExitCode::SUCCESS)
}
