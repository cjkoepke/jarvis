mod transcript;

use crate::agents::claude::Claude;
use crate::agents::hooks::{HookInput, Provider, Reply, Trigger};
use crate::standards::tiers::{self, Tier};
use crate::standards::Standards;
use anyhow::{Context, Result};

/// `Task` is the name older Claude Code versions gave the `Agent` tool.
pub const PROVIDER: Provider = Provider {
    agent: &Claude,
    triggers: &[Trigger {
        event: "PreToolUse",
        matcher: Some("Agent|Task|Workflow"),
    }],
    reply: hold,
};

const ASK_FOR_A_TIER: &str = "No tier is picked for starting agents. Load the `orchestration` skill, then ask the owner with \
AskUserQuestion which tier to use, one option per tier, each label starting with the tier's name:";
const ASK_FOR_A_TYPED_TIER: &str =
    "No tier is picked for starting agents. Load the `orchestration` skill, show the owner these tiers and ask them to reply with the name of one:";
/// AskUserQuestion offers two to four options (https://code.claude.com/docs/en/agent-sdk/user-input#question-format).
const QUESTION_OPTIONS: std::ops::RangeInclusive<usize> = 2..=4;
const HOW_LONG_IT_HOLDS: &str = "Their pick lets agents start until their next message.";

/// Holds back a call from the main conversation until the owner has picked a tier since their
/// last prompt. A subagent's call goes through: starting that subagent already took a pick, and
/// it has no owner to ask.
fn hold(standards: &Standards) -> Result<Reply> {
    let tiers = tiers::offered(standards);
    if tiers.is_empty() {
        return Ok(Reply::Nothing);
    }
    let input = HookInput::from_stdin()?;
    if input.from_subagent {
        return Ok(Reply::Nothing);
    }
    let transcript = input.transcript.context("Claude Code named no transcript")?;
    if transcript::has_tier_pick(&transcript, &tiers)? {
        return Ok(Reply::Nothing);
    }
    Ok(Reply::Deny { reason: refusal(&tiers) })
}

fn refusal(tiers: &[Tier]) -> String {
    let options: Vec<String> = tiers.iter().map(|tier| format!("- {}", tier.line)).collect();
    let instruction = if QUESTION_OPTIONS.contains(&tiers.len()) {
        ASK_FOR_A_TIER
    } else {
        ASK_FOR_A_TYPED_TIER
    };
    format!("{instruction}\n{}\n{HOW_LONG_IT_HOLDS}", options.join("\n"))
}
