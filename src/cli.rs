use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "jarvis",
    version,
    about = "Puts your working standards into every agent session and reviews changes before push.",
    styles = crate::ui::style::help_styles()
)]
pub struct Cli {
    #[arg(long, global = true, help = "Home directory to install into and read from. Defaults to $HOME.")]
    pub home: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    #[command(about = "Copy the standards, register the agent hooks and the git pre-push review, and install ast-grep if it is missing.")]
    Install(InstallArgs),
    #[command(about = "Bring jarvis up to date with the tool that installed it, then set everything up again.")]
    Update,
    #[command(about = "Remove everything `install` added and nothing else.")]
    Uninstall,
    #[command(about = "Show whether the standards, agent hooks, skills, git review and ast-grep are in place.")]
    Status {
        #[arg(long, value_enum, default_value_t = Format::Table, help = "`table` for a person, `json` for an agent.")]
        format: Format,
    },
    #[command(about = "Copy core files or skills you were given into ~/.jarvis/custom, to change them there.")]
    Clone {
        #[arg(help = "What to copy, such as `skills/review` or `core/core.md`. Without any, a list to tick from.")]
        pieces: Vec<String>,
    },
    #[command(about = "Choose defaults such as the pre-push reviewer with the arrow keys.")]
    Settings,
    #[command(about = "Let the reviewer fix what it finds in the repository you are in, or check that it may.")]
    Repo {
        #[command(subcommand)]
        command: RepoCmd,
    },
    #[command(hide = true)]
    Hook { agent: String, event: String },
    #[command(hide = true, name = "pre-push")]
    PrePush {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        git_args: Vec<String>,
    },
}

impl Cmd {
    pub fn runs_inside_agent_or_git(&self) -> bool {
        matches!(self, Cmd::Hook { .. } | Cmd::PrePush { .. })
    }
}

#[derive(Subcommand)]
pub enum RepoCmd {
    #[command(about = "Trust this repository in each agent and give a fixing reviewer the permissions it needs.")]
    Init {
        #[arg(long, short, help = "Trust the repository without being asked.")]
        yes: bool,
    },
    #[command(
        about = "Show whether each agent lets a fixing reviewer edit and commit in this repository, and offer to fix what is missing."
    )]
    Audit {
        #[arg(long, value_enum, default_value_t = Format::Table, help = "`table` for a person, `json` for an agent.")]
        format: Format,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Table,
    Json,
}

#[derive(Args)]
pub struct InstallArgs {
    #[arg(
        long,
        help = "Folder holding core/ and skills/, used in place of the packaged standards. Defaults to the last install's folder, then the packaged standards."
    )]
    pub standards: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "standards",
        help = "Go back to the standards packaged with jarvis after an install from a folder."
    )]
    pub packaged: bool,
    #[arg(
        long,
        value_delimiter = ',',
        help = "Agents to set up, comma-separated. Defaults to the last install's agents, then all of them."
    )]
    pub agents: Vec<String>,
}
