use super::claude_transcript;
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
const ASK_FOR_A_TYPED_TIER: &str =
    "No tier is picked for starting agents. Load the `orchestration` skill, show the owner these tiers and ask them to reply with the name of one:";
/// AskUserQuestion offers two to four options (https://code.claude.com/docs/en/agent-sdk/user-input#question-format).
const QUESTION_OPTIONS: std::ops::RangeInclusive<usize> = 2..=4;
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
    if claude_transcript::has_tier_pick(&input.transcript_path, tiers)? {
        return Ok(None);
    }
    Ok(Some(denial(tiers)))
}

fn denial(tiers: &[Tier]) -> String {
    let options: Vec<String> = tiers.iter().map(|tier| format!("- {}", tier.line)).collect();
    let instruction = if QUESTION_OPTIONS.contains(&tiers.len()) {
        ASK_FOR_A_TIER
    } else {
        ASK_FOR_A_TYPED_TIER
    };
    let reason = format!("{instruction}\n{}\n{HOW_LONG_IT_HOLDS}", options.join("\n"));
    json!({
        "hookSpecificOutput": {
            "hookEventName": HOOK.name,
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    })
    .to_string()
}
