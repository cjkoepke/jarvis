use crate::git;
use crate::paths::Env;
use anyhow::Result;
use std::path::PathBuf;

/// @agent A hook defined in git config runs in addition to a repository's own
/// hooks, so husky, lefthook and pre-commit keep working. Do not switch to
/// `core.hooksPath`: it silently disables the hooks in `.git/hooks`.
const SECTION: &str = "hook.jarvis";
const EVENT: &str = "pre-push";

/// @agent Hooks defined in git config arrived in Git 2.54. Older versions ignore
/// the section, so the review never runs there.
pub const MINIMUM_GIT_VERSION: (u32, u32) = (2, 54);

pub enum Registration {
    Active { executable: PathBuf },
    Missing,
    WrongEvent,
    SwitchedOff,
}

pub fn register(env: &Env) -> Result<()> {
    config(env, &[&format!("{SECTION}.command"), &env.shell_command(EVENT)])?;
    config(env, &[&format!("{SECTION}.event"), EVENT]).map(drop)
}

pub fn unregister(env: &Env) -> Result<()> {
    let section_exists = value(env, "command").is_some() || value(env, "event").is_some();
    if !section_exists {
        return Ok(());
    }
    config(env, &["--remove-section", SECTION]).map(drop)
}

pub fn registration(env: &Env) -> Registration {
    let executable = value(env, "command").and_then(|command| env.executable_in(&command, EVENT));
    let Some(executable) = executable else {
        return Registration::Missing;
    };
    if value(env, "event").as_deref() != Some(EVENT) {
        return Registration::WrongEvent;
    }
    let switched_off = config(env, &["--type=bool", "--get", &format!("{SECTION}.enabled")]).is_ok_and(|enabled| enabled == "false");
    if switched_off {
        return Registration::SwitchedOff;
    }
    Registration::Active { executable }
}

pub fn installed_git_version() -> Option<(u32, u32)> {
    let version_line = git::run(&["--version"]).ok()?;
    let mut numbers = version_line.split_whitespace().nth(2)?.split('.');
    Some((numbers.next()?.parse().ok()?, numbers.next()?.parse().ok()?))
}

fn value(env: &Env, key: &str) -> Option<String> {
    if !env.global_gitconfig().exists() {
        return None;
    }
    config(env, &["--get", &format!("{SECTION}.{key}")]).ok()
}

fn config(env: &Env, args: &[&str]) -> Result<String> {
    let gitconfig = env.global_gitconfig();
    let gitconfig = gitconfig.to_string_lossy();
    let mut command = vec!["config", "--file", &gitconfig];
    command.extend_from_slice(args);
    git::run(&command)
}
