use crate::git;
use crate::paths::Env;
use anyhow::Result;

/// @agent A hook defined in git config runs in addition to a repository's own
/// hooks, so husky, lefthook and pre-commit keep working. Do not switch to
/// `core.hooksPath`: it silently disables the hooks in `.git/hooks`.
const SECTION: &str = "hook.jarvis";

/// @agent Hooks defined in git config arrived in Git 2.54. Older versions ignore
/// the section, so the review never runs there.
pub const MINIMUM_GIT_VERSION: (u32, u32) = (2, 54);

pub fn register(env: &Env) -> Result<()> {
    set(env, "command", &env.shell_command("pre-push"))?;
    set(env, "event", "pre-push")
}

pub fn unregister(env: &Env) -> Result<()> {
    if registered_command(env).is_none() {
        return Ok(());
    }
    config(env, &["--remove-section", SECTION]).map(drop)
}

pub fn is_registered(env: &Env) -> bool {
    registered_command(env).is_some_and(|command| command == env.shell_command("pre-push"))
}

pub fn installed_git_version() -> Option<(u32, u32)> {
    let version_line = git::run(&["--version"]).ok()?;
    let mut numbers = version_line.split_whitespace().nth(2)?.split('.');
    Some((numbers.next()?.parse().ok()?, numbers.next()?.parse().ok()?))
}

fn registered_command(env: &Env) -> Option<String> {
    if !env.global_gitconfig().exists() {
        return None;
    }
    config(env, &["--get", &format!("{SECTION}.command")]).ok()
}

fn set(env: &Env, key: &str, value: &str) -> Result<()> {
    config(env, &[&format!("{SECTION}.{key}"), value]).map(drop)
}

fn config(env: &Env, args: &[&str]) -> Result<String> {
    let gitconfig = env.global_gitconfig();
    let gitconfig = gitconfig.to_string_lossy();
    let mut command = vec!["config", "--file", &gitconfig];
    command.extend_from_slice(args);
    git::run(&command)
}
