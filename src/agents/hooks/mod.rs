mod core_delivery;
pub mod file;
mod input;
mod launch_gate;
mod reply;

pub use input::HookInput;
pub use reply::Reply;

use super::Agent;
use crate::standards::Standards;
use anyhow::Result;

/// Something jarvis has agents run at set moments of a session.
pub struct Hook {
    pub name: &'static str,
    pub providers: &'static [Provider],
}

/// One hook as one agent runs it. A reply that needs to know about the moment asks
/// `HookInput::from_stdin`.
pub struct Provider {
    pub agent: &'static dyn Agent,
    pub triggers: &'static [Trigger],
    pub reply: fn(&Standards) -> Result<Reply>,
}

/// An event of the agent's and, for a tool event, the tools it is for, in the agent's own
/// matcher syntax.
pub struct Trigger {
    pub event: &'static str,
    pub matcher: Option<&'static str>,
}

/// One handler jarvis keeps in an agent's hooks file.
pub struct Placement {
    pub hook: &'static Hook,
    pub provider: &'static Provider,
    pub trigger: &'static Trigger,
}

pub fn all() -> Vec<&'static Hook> {
    vec![&core_delivery::HOOK, &launch_gate::HOOK]
}

pub fn placements(agent: &dyn Agent) -> Vec<Placement> {
    let mut placements = Vec::new();
    for hook in all() {
        for provider in hook.providers.iter().filter(|provider| provider.agent.id() == agent.id()) {
            for trigger in provider.triggers {
                placements.push(Placement { hook, provider, trigger });
            }
        }
    }
    placements
}

/// The handler a registered `jarvis hook <agent> <event>` runs.
pub fn placed_on(agent: &dyn Agent, event: &str) -> Option<Placement> {
    placements(agent).into_iter().find(|placement| placement.trigger.event == event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents;

    #[test]
    fn an_agent_event_runs_one_hook_so_the_registered_command_names_it() {
        for agent in agents::all() {
            let mut events: Vec<&str> = placements(agent).iter().map(|placement| placement.trigger.event).collect();
            let placed = events.len();
            events.sort_unstable();
            events.dedup();
            assert_eq!(events.len(), placed, "{} has two hooks on one event", agent.id());
        }
    }
}
