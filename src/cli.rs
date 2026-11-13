use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "jarvis",
    version,
    about = "Puts your working standards into every agent session and reviews changes before push."
)]
pub struct Cli {
    #[arg(long, global = true, help = "Home directory to install into and read from. Defaults to $HOME.")]
    pub home: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    #[command(about = "Copy the standards, register the agent hooks and the git pre-push review.")]
    Install(InstallArgs),
    #[command(about = "Remove everything `install` added and nothing else.")]
    Uninstall {
        #[arg(long, help = "Allow changing your real home directory.")]
        real: bool,
    },
    #[command(about = "Check that the standards, agent hooks, skills and git review are in place.")]
    Doctor,
    #[command(hide = true)]
    Hook { agent: String, event: String },
    #[command(hide = true, name = "pre-push")]
    PrePush {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        git_args: Vec<String>,
    },
}

impl Cmd {
    /// @agent These commands run inside an agent session or a `git push`. They must
    /// exit 0 on their own failures so jarvis is never the reason either one breaks.
    pub fn runs_inside_agent_or_git(&self) -> bool {
        matches!(self, Cmd::Hook { .. } | Cmd::PrePush { .. })
    }
}

#[derive(Args)]
pub struct InstallArgs {
    #[arg(long, default_value = "standards", help = "Folder holding core/, skills/ and rules/.")]
    pub standards: PathBuf,
    #[arg(long, value_delimiter = ',', help = "Agents to set up, comma-separated. Defaults to all of them.")]
    pub agents: Vec<String>,
    #[arg(long, help = "Allow writing into your real home directory.")]
    pub real: bool,
}
