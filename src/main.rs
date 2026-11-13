mod agents;
mod check;
mod cli;
mod commands;
mod files;
mod git;
mod paths;
mod rules;
mod standards;

use clap::Parser;
use cli::{Cli, Cmd};
use paths::Env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let runs_inside_agent_or_git = cli.command.runs_inside_agent_or_git();

    let env = match Env::new(cli.home) {
        Ok(env) => env,
        Err(error) => {
            eprintln!("jarvis: {error:#}");
            return if runs_inside_agent_or_git {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
    };

    let outcome = match cli.command {
        Cmd::Install(args) => commands::install::run(&env, &args),
        Cmd::Uninstall { real } => commands::uninstall::run(&env, real),
        Cmd::Doctor => commands::doctor::run(&env),
        Cmd::Hook { agent, event } => Ok(commands::hook::run(&env, &agent, &event)),
        Cmd::PrePush { git_args } => Ok(commands::pre_push::run(&env, &git_args)),
    };
    outcome.unwrap_or_else(|error| {
        eprintln!("jarvis: {error:#}");
        ExitCode::FAILURE
    })
}
