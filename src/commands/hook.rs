use crate::agents;
use crate::paths::Env;
use crate::standards::Standards;
use crate::ui::message;
use std::process::ExitCode;

pub fn run(env: &Env, agent_id: &str, event: &str) -> ExitCode {
    let Some(agent) = agents::by_id(agent_id) else {
        message::error(&format!("unknown agent `{agent_id}`"));
        return ExitCode::SUCCESS;
    };
    match Standards::load(&env.installed_standards_dir()) {
        Ok(standards) => println!("{}", agent.hook_reply(event, &standards.core)),
        Err(error) => message::error(&format!("no standards delivered ({error:#})")),
    }
    ExitCode::SUCCESS
}
