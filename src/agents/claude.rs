use super::Agent;
use crate::paths::Env;
use std::path::PathBuf;

/// @agent `SubagentStart` is registered as well as `SessionStart` because Claude
/// Code's built-in Explore and Plan sub-agents skip CLAUDE.md, so a hook is the only
/// way the standards reach them (https://code.claude.com/docs/en/hooks).
pub struct Claude;

impl Agent for Claude {
    fn id(&self) -> &'static str {
        "claude"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn executable_name(&self) -> &'static str {
        "claude"
    }

    fn hooks_file(&self, env: &Env) -> PathBuf {
        env.home.join(".claude/settings.json")
    }

    fn skills_dir(&self, env: &Env) -> PathBuf {
        env.home.join(".claude/skills")
    }

    fn context_events(&self) -> &'static [&'static str] {
        &["SessionStart", "SubagentStart"]
    }
}
