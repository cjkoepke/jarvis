use super::{hook_command, hook_subcommand, Agent};
use crate::paths::Env;
use anyhow::{bail, Context, Result};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

type Config = Map<String, Value>;

pub fn register(agent: &dyn Agent, env: &Env) -> Result<()> {
    let path = agent.hooks_file(env);
    let mut config = read(&path)?;
    remove_jarvis_hooks(agent, &mut config);

    let hooks = config.entry("hooks").or_insert_with(|| json!({}));
    let hooks = hooks
        .as_object_mut()
        .with_context(|| format!("`hooks` in {} is not an object", path.display()))?;
    for event in agent.context_events() {
        let groups = hooks.entry(*event).or_insert_with(|| json!([]));
        let groups = groups
            .as_array_mut()
            .with_context(|| format!("`hooks.{event}` in {} is not a list", path.display()))?;
        groups.push(json!({ "hooks": [{ "type": "command", "command": hook_command(agent, env, event) }] }));
    }
    write(&path, &config)
}

pub fn unregister(agent: &dyn Agent, env: &Env) -> Result<()> {
    let path = agent.hooks_file(env);
    if !path.exists() {
        return Ok(());
    }
    let mut config = read(&path)?;
    if !remove_jarvis_hooks(agent, &mut config) {
        return Ok(());
    }
    if config.is_empty() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))
    } else {
        write(&path, &config)
    }
}

pub struct RegisteredHook {
    pub event: &'static str,
    pub executable: PathBuf,
}

pub fn registered(agent: &dyn Agent, env: &Env) -> Vec<RegisteredHook> {
    let config = read(&agent.hooks_file(env)).unwrap_or_default();
    let mut registered = Vec::new();
    for event in agent.context_events().iter().copied() {
        let subcommand = hook_subcommand(agent, event);
        let groups = config.get("hooks").and_then(|hooks| hooks.get(event)).and_then(Value::as_array);
        let handlers = groups
            .into_iter()
            .flatten()
            .filter_map(|group| group.get("hooks").and_then(Value::as_array))
            .flatten();
        for handler in handlers {
            if let Some(executable) = command_of(handler).and_then(|command| env.executable_in(command, &subcommand)) {
                registered.push(RegisteredHook { event, executable });
            }
        }
    }
    registered
}

fn command_of(handler: &Value) -> Option<&str> {
    handler.get("command").and_then(Value::as_str)
}

/// @agent Matches loosely instead of on the exact command, so hooks written by a
/// jarvis binary at an older path are still found and replaced.
fn is_jarvis_hook(agent: &dyn Agent, handler: &Value) -> bool {
    command_of(handler).is_some_and(|command| command.contains("jarvis") && command.contains(&format!(" hook {} ", agent.id())))
}

/// Returns whether anything was removed.
fn remove_jarvis_hooks(agent: &dyn Agent, config: &mut Config) -> bool {
    let Some(hooks) = config.get_mut("hooks").and_then(Value::as_object_mut) else {
        return false;
    };

    let mut removed_any = false;
    for groups in hooks.values_mut().filter_map(Value::as_array_mut) {
        groups.retain_mut(|group| {
            let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };
            let handlers_before = handlers.len();
            handlers.retain(|handler| !is_jarvis_hook(agent, handler));
            let removed_here = handlers.len() != handlers_before;
            removed_any |= removed_here;
            let group_is_now_empty = removed_here && handlers.is_empty();
            !group_is_now_empty
        });
    }

    if removed_any {
        hooks.retain(|_, groups| !groups.as_array().is_some_and(|groups| groups.is_empty()));
        if hooks.is_empty() {
            config.shift_remove("hooks");
        }
    }
    removed_any
}

/// @agent Anything that is not a JSON object is an error on purpose: jarvis must
/// never overwrite agent configuration it could not read.
fn read(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::new());
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Config::new());
    }
    match serde_json::from_str(&text).with_context(|| format!("{} is not valid JSON", path.display()))? {
        Value::Object(config) => Ok(config),
        _ => bail!("{} does not hold a JSON object", path.display()),
    }
}

fn write(path: &Path, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(config)? + "\n";
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}
