use super::{claude_models, Agent, ReviewModel, ReviewRun};
use crate::paths::Env;
use std::path::PathBuf;
use std::process::Command;

const REVIEW_TOOLS: &str = "Read,Grep,Glob,Bash";
const REVIEW_COMMANDS: [&str; 4] = ["Bash(git log *)", "Bash(git show *)", "Bash(git diff *)", "Bash(printenv *)"];

const FIX_COMMANDS: [&str; 5] = [
    "Bash(sed *)",
    "Bash(git status *)",
    "Bash(git add *)",
    "Bash(git commit *)",
    "Bash(git restore *)",
];
const STAY_IN_THE_REPO_VARIABLE: &str = "CLAUDE_BASH_MAINTAIN_PROJECT_WORKING_DIR";

pub struct Claude;

impl Claude {
    fn headless_command(&self, run: &ReviewRun, allowed: &[String]) -> Command {
        let mut command = Command::new(self.executable_name());
        command
            .args(["--print", "--safe-mode", "--no-session-persistence", "--permission-prompts", "none"])
            .arg("--add-dir")
            .arg(run.standards_dir)
            .arg("--allowedTools")
            .args(allowed)
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
        let mut command = self.headless_command(run, &REVIEW_COMMANDS.map(String::from));
        command.args(["--tools", REVIEW_TOOLS]);
        command
    }

    fn fixing_review_command(&self, run: &ReviewRun) -> Command {
        let edits_in_the_repo = format!("Edit(/{}/**)", run.repo.dir.display());
        let allowed: Vec<String> = REVIEW_COMMANDS
            .iter()
            .chain(&FIX_COMMANDS)
            .map(|rule| rule.to_string())
            .chain([edits_in_the_repo])
            .collect();
        self.headless_command(run, &allowed)
    }
}
