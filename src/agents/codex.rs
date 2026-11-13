use super::Agent;
use crate::paths::Env;
use std::path::PathBuf;

/// @agent Codex reads `~/.codex/hooks.json` in the same shape Claude Code uses, but
/// ignores a hook from that file until the person approves it once
/// (https://learn.chatgpt.com/docs/hooks). jarvis cannot grant or detect that approval.
pub struct Codex;

impl Agent for Codex {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn executable_name(&self) -> &'static str {
        "codex"
    }

    fn hooks_file(&self, env: &Env) -> PathBuf {
        env.home.join(".codex/hooks.json")
    }

    fn skills_dir(&self, env: &Env) -> PathBuf {
        env.home.join(".agents/skills")
    }

    fn context_events(&self) -> &'static [&'static str] {
        &["SessionStart", "SubagentStart"]
    }

    fn manual_step_after_install(&self) -> Option<&'static str> {
        Some("open Codex, run /hooks and approve the jarvis hooks once; Codex ignores them until then")
    }
}
