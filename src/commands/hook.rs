use crate::agents;
use crate::paths::Env;
use crate::standards::Standards;
use std::process::ExitCode;

/// @agent Always succeeds, and prints nothing to stdout when it cannot deliver. A
/// session has to start without the standards rather than fail because of jarvis.
pub fn run(env: &Env, agent_id: &str, event: &str) -> ExitCode {
    let Some(agent) = agents::by_id(agent_id) else {
        eprintln!("jarvis: unknown agent `{agent_id}`");
        return ExitCode::SUCCESS;
    };
    match Standards::load(&env.installed_standards_dir()) {
        Ok(standards) => {
            let context = if event == "SubagentStart" {
                standards.subagent_context()
            } else {
                standards.session_context()
            };
            println!("{}", agent.hook_reply(event, &context));
        }
        Err(error) => eprintln!("jarvis: no standards delivered ({error:#})"),
    }
    ExitCode::SUCCESS
}
