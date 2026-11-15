use super::{codex_models, codex_trust, Agent, Approval, ReviewModel};
use crate::paths::Env;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    fn review_models(&self, env: &Env) -> Vec<ReviewModel> {
        codex_models::offered(env)
    }

    fn approval(&self, env: &Env) -> Approval {
        codex_trust::approval(self, env)
    }

    fn review_command(&self, _standards_dir: &Path, model: Option<&str>) -> Command {
        let mut command = Command::new(self.executable_name());
        command.args(["exec", "--skip-git-repo-check", "--sandbox", "read-only", "--ephemeral"]);
        if let Some(model) = model {
            command.args(["--model", model]);
        }
        command.arg("-");
        command
    }
}
