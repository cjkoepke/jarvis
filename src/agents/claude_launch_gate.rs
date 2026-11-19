use super::claude_transcript::{self, OwnerTurn};
use super::HookEvent;
use crate::standards::tiers::Tier;
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

/// `Task` is the name older Claude Code versions gave the `Agent` tool.
pub const HOOK: HookEvent = HookEvent {
    name: "PreToolUse",
    matcher: Some("Agent|Task|Workflow"),
};

const ASK_FOR_A_TIER: &str = "No tier is picked for starting agents. Load the `orchestration` skill, then ask the owner with \
AskUserQuestion which tier to use, one option per tier, each label starting with the tier's name:";
const HOW_LONG_IT_HOLDS: &str = "Their pick lets agents start until their next message.";

#[derive(Deserialize)]
struct HookInput {
    transcript_path: PathBuf,
    /// Set when the call comes from a subagent or a teammate, never from the main conversation.
    agent_id: Option<String>,
}

/// Holds back a call from the main conversation until the owner has picked a tier since their
/// last prompt. A subagent's call goes through: starting that subagent already took a pick, and
/// it has no owner to ask.
pub fn hold(hook_input: &str, tiers: &[Tier]) -> Result<Option<String>> {
    let input: HookInput = serde_json::from_str(hook_input)?;
    if tiers.is_empty() || input.agent_id.is_some() {
        return Ok(None);
    }
    let turn = claude_transcript::owner_turn(&input.transcript_path)?;
    if picks_a_tier(&turn, tiers) {
        return Ok(None);
    }
    Ok(Some(denial(tiers)))
}

fn picks_a_tier(turn: &OwnerTurn, tiers: &[Tier]) -> bool {
    tiers.iter().any(|tier| {
        let in_prompt = turn.prompt.as_deref().is_some_and(|prompt| tier.is_named_in_prompt(prompt));
        in_prompt || turn.answers.iter().any(|answer| tier.is_named_in_answer(answer))
    })
}

fn denial(tiers: &[Tier]) -> String {
    let options: Vec<String> = tiers.iter().map(|tier| format!("- {}", tier.line)).collect();
    let reason = format!("{ASK_FOR_A_TIER}\n{}\n{HOW_LONG_IT_HOLDS}", options.join("\n"));
    json!({
        "hookSpecificOutput": {
            "hookEventName": HOOK.name,
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    })
    .to_string()
}
