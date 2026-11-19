use crate::agents::{self, Agent};
use crate::paths::Env;
use crate::standards::{tiers, Standards};
use crate::ui::message;
use anyhow::Result;
use std::io::{self, Read};
use std::process::ExitCode;

pub fn run(env: &Env, agent_id: &str, event: &str) -> ExitCode {
    let Some(agent) = agents::by_id(agent_id) else {
        message::error(&format!("unknown agent `{agent_id}`"));
        return ExitCode::SUCCESS;
    };
    let standards = match Standards::load(&env.installed_standards_dir()) {
        Ok(standards) => standards,
        Err(error) => {
            message::error(&format!("no standards delivered ({error:#})"));
            return ExitCode::SUCCESS;
        }
    };
    if agent.launch_gate().is_some_and(|gate| gate.name == event) {
        match hold_launch(agent, &standards) {
            Ok(Some(reply)) => println!("{reply}"),
            Ok(None) => {}
            Err(error) => message::error(&format!("could not check for an orchestration tier ({error:#})")),
        }
    } else {
        println!("{}", agent.hook_reply(event, &standards.core));
    }
    ExitCode::SUCCESS
}

fn hold_launch(agent: &dyn Agent, standards: &Standards) -> Result<Option<String>> {
    let mut hook_input = String::new();
    io::stdin().read_to_string(&mut hook_input)?;
    agent.hold_launch(&hook_input, &tiers::offered(standards))
}
