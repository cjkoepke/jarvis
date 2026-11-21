use serde_json::json;

/// What a hook answers, whichever agent asked.
pub enum Reply {
    /// Text the agent puts in the model's context.
    Context(String),
    /// The tool call does not run, and the model is told why.
    Deny { reason: String },
    /// The agent carries on as if the hook were not there.
    Nothing,
}

impl Reply {
    /// The line the agent reads on the hook's stdout, in the shape Claude Code and Codex share
    /// (https://code.claude.com/docs/en/hooks, https://learn.chatgpt.com/docs/hooks).
    pub fn on_the_wire(&self, event: &str) -> Option<String> {
        let answer = match self {
            Reply::Context(context) => json!({ "hookEventName": event, "additionalContext": context }),
            Reply::Deny { reason } => json!({ "hookEventName": event, "permissionDecision": "deny", "permissionDecisionReason": reason }),
            Reply::Nothing => return None,
        };
        Some(json!({ "hookSpecificOutput": answer }).to_string())
    }
}
