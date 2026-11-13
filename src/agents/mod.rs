pub mod checks;
mod claude;
mod codex;
pub mod hooks_file;
pub mod skills;

use crate::paths::Env;
use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;

/// @agent To support another agent, add one file implementing this trait and list
/// it in `all()`. Hook registration, skills and status checks need no changes.
pub trait Agent: Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn executable_name(&self) -> &'static str;
    fn hooks_file(&self, env: &Env) -> PathBuf;
    fn skills_dir(&self, env: &Env) -> PathBuf;
    fn context_events(&self) -> &'static [&'static str];

    fn manual_step_after_install(&self) -> Option<&'static str> {
        None
    }

    fn hook_reply(&self, event: &str, context: &str) -> String {
        json!({ "hookSpecificOutput": { "hookEventName": event, "additionalContext": context } }).to_string()
    }
}

pub fn all() -> Vec<&'static dyn Agent> {
    vec![&claude::Claude, &codex::Codex]
}

pub fn by_id(id: &str) -> Option<&'static dyn Agent> {
    all().into_iter().find(|agent| agent.id() == id)
}

pub fn chosen(ids: &[String]) -> Result<Vec<&'static dyn Agent>> {
    if ids.is_empty() {
        return Ok(all());
    }
    let known_ids: Vec<&str> = all().iter().map(|agent| agent.id()).collect();
    ids.iter()
        .map(|id| by_id(id).with_context(|| format!("unknown agent `{id}`; known agents: {}", known_ids.join(", "))))
        .collect()
}

pub fn hook_subcommand(agent: &dyn Agent, event: &str) -> String {
    format!("hook {} {event}", agent.id())
}

pub fn hook_command(agent: &dyn Agent, env: &Env, event: &str) -> String {
    env.shell_command(&hook_subcommand(agent, event))
}
