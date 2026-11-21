use anyhow::{Context, Result};
use serde::Deserialize;
use std::io::{self, Read};
use std::path::PathBuf;

/// What a hook is told about the moment it runs, whichever agent ran it.
pub struct HookInput {
    pub transcript: Option<PathBuf>,
    /// Whether a subagent or a teammate made the call, not the main conversation.
    pub from_subagent: bool,
}

/// What jarvis reads of the JSON an agent writes to a hook's stdin. Claude Code and Codex name
/// these fields alike (https://code.claude.com/docs/en/hooks, https://learn.chatgpt.com/docs/hooks).
#[derive(Deserialize)]
struct Payload {
    transcript_path: Option<PathBuf>,
    agent_id: Option<String>,
}

impl HookInput {
    /// Waits until the agent closes the hook's stdin, so only a hook that needs the input asks for it.
    pub fn from_stdin() -> Result<HookInput> {
        let mut payload = String::new();
        io::stdin().read_to_string(&mut payload).context("reading the agent's hook input")?;
        let payload: Payload = serde_json::from_str(&payload).context("reading the agent's hook input")?;
        Ok(HookInput {
            transcript: payload.transcript_path,
            from_subagent: payload.agent_id.is_some(),
        })
    }
}
