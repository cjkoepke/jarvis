use super::Agent;
use crate::paths::Env;
use std::path::{Path, PathBuf};
use std::process::Command;

const READ_ONLY_TOOLS: &str = "Read,Grep,Glob";

/// @agent `SubagentStart` is registered as well as `SessionStart` because Claude
/// Code's built-in Explore and Plan sub-agents skip CLAUDE.md, so a hook is the only
/// way the standards reach them (https://code.claude.com/docs/en/sub-agents).
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

    fn review_command(&self, standards_dir: &Path, model: Option<&str>) -> Command {
        let mut command = Command::new(self.executable_name());
        command
            .args(["--print", "--tools", READ_ONLY_TOOLS])
            .args(["--safe-mode", "--no-session-persistence", "--permission-prompts", "none"])
            .arg("--add-dir")
            .arg(standards_dir);
        if let Some(model) = model {
            command.args(["--model", model]);
        }
        command
    }
}
