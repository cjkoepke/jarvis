use super::{codex_models, codex_trust, Agent, Approval, ReviewModel, ReviewRun};
use crate::paths::Env;
use std::path::PathBuf;
use std::process::Command;

const READ_ONLY: &str = "read-only";
const WRITES_IN_THE_REPO: &str = "workspace-write";
const REACH_THE_SIGNING_AGENT: &str = "sandbox_workspace_write.network_access=true";
const PROMPT_ON_STDIN: &str = "-";

pub struct Codex;

impl Codex {
    fn headless_command(&self, sandbox: &str, run: &ReviewRun) -> Command {
        let mut command = Command::new(self.executable_name());
        command.args(["exec", "--skip-git-repo-check", "--ephemeral", "--sandbox", sandbox]);
        if let Some(model) = run.model {
            command.args(["--model", model]);
        }
        command
    }
}

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

    fn review_command(&self, run: &ReviewRun) -> Command {
        let mut command = self.headless_command(READ_ONLY, run);
        command.arg(PROMPT_ON_STDIN);
        command
    }

    fn fixing_review_command(&self, run: &ReviewRun) -> Command {
        let mut command = self.headless_command(WRITES_IN_THE_REPO, run);
        for git_dir in &run.repo.git_dirs {
            command.arg("--add-dir").arg(git_dir);
        }
        if run.repo.signs_commits {
            command.args(["--config", REACH_THE_SIGNING_AGENT]);
        }
        command.arg(PROMPT_ON_STDIN);
        command
    }
}
