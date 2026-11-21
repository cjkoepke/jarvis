use super::{codex_models, codex_repo, codex_trust, Agent, Approval, ReviewModel, ReviewRun};
use crate::check::requirement::Requirement;
use crate::git::repo::Repo;
use crate::paths::Env;
use std::path::PathBuf;
use std::process::Command;

const READ_ONLY: &str = "read-only";
const REACH_THE_SIGNING_AGENT: &str = "sandbox_workspace_write.network_access=true";
const PROMPT_ON_STDIN: &str = "-";

pub struct Codex;

pub(super) fn config_file(env: &Env) -> PathBuf {
    env.home.join(".codex/config.toml")
}

impl Codex {
    fn headless_command(&self, run: &ReviewRun) -> Command {
        let mut command = Command::new(self.executable_name());
        command.args(["exec", "--skip-git-repo-check", "--ephemeral"]);
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

    fn review_models(&self, env: &Env) -> Vec<ReviewModel> {
        codex_models::offered(env)
    }

    fn approval(&self, env: &Env) -> Approval {
        codex_trust::approval(self, env)
    }

    fn review_command(&self, run: &ReviewRun) -> Command {
        let mut command = self.headless_command(run);
        command.args(["--sandbox", READ_ONLY, PROMPT_ON_STDIN]);
        command
    }

    fn fixing_review_command(&self, run: &ReviewRun) -> Command {
        if !self.fixing_run_may_write(run.env, run.repo) {
            return self.review_command(run);
        }
        let mut command = self.headless_command(run);
        for git_dir in &run.repo.git_dirs {
            command.arg("--add-dir").arg(git_dir);
        }
        if run.repo.signs_commits {
            command.args(["--config", REACH_THE_SIGNING_AGENT]);
        }
        command.arg(PROMPT_ON_STDIN);
        command
    }

    fn fixing_run_may_write(&self, env: &Env, repo: &Repo) -> bool {
        codex_repo::is_trusted(env, repo)
    }

    fn fixing_requirements<'a>(&self, env: &'a Env, repo: &'a Repo) -> Vec<Requirement<'a>> {
        codex_repo::fixing_requirements(self, env, repo)
    }
}
