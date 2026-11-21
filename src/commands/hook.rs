use crate::agents;
use crate::agents::hooks;
use crate::paths::Env;
use crate::standards::Standards;
use crate::ui::message;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::process::ExitCode;

pub fn run(env: &Env, agent_id: &str, event: &str) -> ExitCode {
    if let Err(error) = answer(env, agent_id, event) {
        message::error(&format!("{error:#}"));
    }
    ExitCode::SUCCESS
}

fn answer(env: &Env, agent_id: &str, event: &str) -> Result<()> {
    let agent = agents::by_id(agent_id).with_context(|| format!("unknown agent `{agent_id}`"))?;
    let placement = hooks::placed_on(agent, event).with_context(|| format!("jarvis has no hook on {event} in {}", agent.display_name()))?;
    let standards = Standards::load(&env.installed_standards_dir()).context("no standards delivered")?;
    let reply = (placement.provider.reply)(&standards).with_context(|| format!("the {} hook could not answer", placement.hook.name))?;
    if let Some(line) = reply.on_the_wire(event) {
        writeln!(io::stdout(), "{line}").context("the agent stopped reading the hook's reply")?;
    }
    Ok(())
}
