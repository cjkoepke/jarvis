use super::json_config::{self, Config};
use super::{hook_command, hook_events, hook_subcommand, Agent};
use crate::paths::Env;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

const TITLE: &str = "jarvis: working standards";

pub fn register(agent: &dyn Agent, env: &Env) -> Result<()> {
    let path = agent.hooks_file(env);
    let mut config = json_config::read(&path)?;
    remove_jarvis_hooks(agent, &mut config);

    let hooks = config.entry("hooks").or_insert_with(|| json!({}));
    let hooks = hooks
        .as_object_mut()
        .with_context(|| format!("`hooks` in {} is not an object", path.display()))?;
    for event in hook_events(agent) {
        let groups = hooks.entry(event.name).or_insert_with(|| json!([]));
        let groups = groups
            .as_array_mut()
            .with_context(|| format!("`hooks.{}` in {} is not a list", event.name, path.display()))?;
        let handler = json!({ "type": "command", "command": hook_command(agent, env, event.name), "statusMessage": TITLE });
        groups.push(match event.matcher {
            Some(matcher) => json!({ "matcher": matcher, "hooks": [handler] }),
            None => json!({ "hooks": [handler] }),
        });
    }
    json_config::write(&path, &config)
}

pub fn unregister(agent: &dyn Agent, env: &Env) -> Result<()> {
    let path = agent.hooks_file(env);
    if !path.exists() {
        return Ok(());
    }
    let mut config = json_config::read(&path)?;
    if !remove_jarvis_hooks(agent, &mut config) {
        return Ok(());
    }
    if config.is_empty() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))
    } else {
        json_config::write(&path, &config)
    }
}

pub struct RegisteredHook {
    pub event: &'static str,
    pub group_index: usize,
    pub handler_index: usize,
    pub executable: PathBuf,
}

pub fn registered(agent: &dyn Agent, env: &Env) -> Vec<RegisteredHook> {
    let config = json_config::read(&agent.hooks_file(env)).unwrap_or_default();
    let mut registered = Vec::new();
    for event in hook_events(agent).into_iter().map(|event| event.name) {
        let subcommand = hook_subcommand(agent, event);
        let groups = config.get("hooks").and_then(|hooks| hooks.get(event)).and_then(Value::as_array);
        for (group_index, group) in groups.into_iter().flatten().enumerate() {
            let handlers = group.get("hooks").and_then(Value::as_array);
            for (handler_index, handler) in handlers.into_iter().flatten().enumerate() {
                let executable = command_of(handler).and_then(|command| env.executable_in(command, &subcommand));
                if let Some(executable) = executable {
                    registered.push(RegisteredHook {
                        event,
                        group_index,
                        handler_index,
                        executable,
                    });
                }
            }
        }
    }
    registered
}

fn command_of(handler: &Value) -> Option<&str> {
    handler.get("command").and_then(Value::as_str)
}

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
