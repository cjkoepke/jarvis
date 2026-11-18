use crate::agents::{self, hooks_file, skills};
use crate::ast_grep::{self, Action, PackageManager};
use crate::git::hook_config;
use crate::paths::Env;
use crate::record::InstallRecord;
use crate::ui::message;
use anyhow::Result;
use std::fs;
use std::process::ExitCode;

pub fn run(env: &Env) -> Result<ExitCode> {
    let ast_grep_installed_by = InstallRecord::load(env)
        .map(|record| record.ast_grep_installed_by)
        .unwrap_or_default();
    for agent in agents::all() {
        hooks_file::unregister(agent, env)?;
        skills::remove(agent, env)?;
    }
    hook_config::unregister(env)?;
    if !ast_grep_installed_by.is_empty() {
        remove_ast_grep(env, &ast_grep_installed_by);
    }
    remove_what_jarvis_wrote_for_itself(env)?;

    message::done("jarvis", &format!("removed from {}", env.home.display()));
    let custom_dir = env.custom_standards_dir();
    if custom_dir.exists() {
        message::note(&format!("your own standards stay in {}", custom_dir.display()));
    }
    Ok(ExitCode::SUCCESS)
}

/// A package manager can report success and remove nothing, as Homebrew does for a pinned
/// formula, so PATH has the last word.
fn remove_ast_grep(env: &Env, installed_by: &[PackageManager]) {
    let mut every_removal_finished = true;
    for manager in installed_by {
        message::caution(
            "ast-grep",
            &format!("an install put it here; running `{}`", manager.command_line(Action::Uninstall)),
        );
        if let Err(error) = manager.run(Action::Uninstall, &env.home) {
            message::note(&format!("{error:#}, so that ast-grep stays"));
            every_removal_finished = false;
        }
    }
    match ast_grep::on_path() {
        Some(remaining) => message::note(&format!("`ast-grep` is still at {}", remaining.display())),
        None if every_removal_finished => message::done("ast-grep", "removed"),
        None => {}
    }
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
