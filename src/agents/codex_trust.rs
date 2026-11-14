use super::hooks_file::{self, RegisteredHook};
use super::{Agent, Approval};
use crate::paths::Env;
use std::fs;
use std::path::Path;
use toml::{Table, Value};

const HOW_TO_APPROVE: &str = "open Codex, run /hooks and approve the jarvis hooks; Codex ignores them until then";
const HOW_TO_ENABLE: &str = "open Codex, run /hooks and switch the jarvis hooks back on";

/// @agent https://github.com/openai/codex/blob/main/codex-rs/hooks/src/lib.rs (`hook_key`)
pub fn approval(agent: &dyn Agent, env: &Env) -> Approval {
    let hooks = hooks_file::registered(agent, env);
    let states = hook_states(env).unwrap_or_default();
    let hooks_file = agent.hooks_file(env);
    let state_of = |hook: &RegisteredHook| states.get(&state_key(&hooks_file, hook));

    let all_approved = hooks
        .iter()
        .all(|hook| state_of(hook).is_some_and(|state| state.get("trusted_hash").is_some()));
    if hooks.is_empty() || !all_approved {
        return Approval::Pending { next_step: HOW_TO_APPROVE };
    }
    let any_switched_off = hooks
        .iter()
        .filter_map(state_of)
        .any(|state| state.get("enabled").and_then(Value::as_bool) == Some(false));
    if any_switched_off {
        return Approval::Pending { next_step: HOW_TO_ENABLE };
    }
    Approval::Granted
}

fn hook_states(env: &Env) -> Option<Table> {
    let config: Table = fs::read_to_string(env.home.join(".codex/config.toml")).ok()?.parse().ok()?;
    config.get("hooks")?.get("state")?.as_table().cloned()
}

fn state_key(hooks_file: &Path, hook: &RegisteredHook) -> String {
    format!(
        "{}:{}:{}:{}",
        hooks_file.display(),
        snake_case(hook.event),
        hook.group_index,
        hook.handler_index
    )
}

fn snake_case(event: &str) -> String {
    let mut snake = String::new();
    for (index, character) in event.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            snake.push('_');
        }
        snake.extend(character.to_lowercase());
    }
    snake
}
