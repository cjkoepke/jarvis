use super::{claude_models, claude_repo, Agent, ReviewModel, ReviewRun};
use crate::check::requirement::Requirement;
use crate::git::repo::Repo;
use crate::paths::Env;
use std::path::PathBuf;
use std::process::Command;

const REVIEW_TOOLS: &str = "Read,Grep,Glob,Bash";
const REVIEW_COMMANDS: [&str; 4] = ["Bash(git log *)", "Bash(git show *)", "Bash(git diff *)", "Bash(printenv *)"];

const STAY_IN_THE_REPO_VARIABLE: &str = "CLAUDE_BASH_MAINTAIN_PROJECT_WORKING_DIR";

pub struct Claude;

impl Claude {
    fn headless_command(&self, run: &ReviewRun) -> Command {
        let mut command = Command::new(self.executable_name());
        command
            .args(["--print", "--safe-mode", "--no-session-persistence", "--permission-prompts", "none"])
            .arg("--add-dir")
            .arg(run.standards_dir)
            .arg("--allowedTools")
            .args(REVIEW_COMMANDS)
            .env(STAY_IN_THE_REPO_VARIABLE, "1");
        if let Some(model) = run.model {
            command.args(["--model", model]);
        }
        command
    }
}

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

    fn review_models(&self, env: &Env) -> Vec<ReviewModel> {
        claude_models::offered(env)
    }

    fn review_command(&self, run: &ReviewRun) -> Command {
        let mut command = self.headless_command(run);
        command.args(["--tools", REVIEW_TOOLS]);
        command
    }

    fn fixing_review_command(&self, run: &ReviewRun) -> Command {
        self.headless_command(run)
    }

    fn fixing_requirements<'a>(&self, env: &'a Env, repo: &'a Repo) -> Vec<Requirement<'a>> {
        claude_repo::fixing_requirements(self, env, repo)
    }
}
