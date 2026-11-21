use crate::standards::tiers::Tier;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Only a typed owner prompt starts a new turn. Background notifications preserve the choice.
pub fn has_tier_pick(transcript: &Path, tiers: &[Tier]) -> Result<bool> {
    let file = File::open(transcript).with_context(|| format!("reading {}", transcript.display()))?;
    let mut picked = false;
    for line in BufReader::new(file).lines() {
        let line = line.with_context(|| format!("reading {}", transcript.display()))?;
        if !may_be_from_the_owner(&line) {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if entry["type"] != "user" || entry["isSidechain"] == true {
            continue;
        }
        let origin = entry["origin"]["kind"].as_str();
        if origin.is_some_and(|kind| kind != "human") {
            continue;
        }
        if entry["toolUseResult"].is_object() {
            picked = picked || answers_pick_tier(&entry["toolUseResult"], tiers);
        } else if origin == Some("human") {
            if let Some(prompt) = typed_prompt(&entry) {
                picked = tiers.iter().any(|tier| tier.is_named_in_prompt(&prompt));
            }
        }
    }
    Ok(picked)
}

/// Transcripts reach hundreds of megabytes, so only lines that can hold a prompt or an answer are parsed.
fn may_be_from_the_owner(line: &str) -> bool {
    line.contains("\"human\"") || line.contains("\"answers\"")
}

fn typed_prompt(entry: &Value) -> Option<String> {
    match &entry["message"]["content"] {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            let texts: Vec<&str> = blocks.iter().filter_map(|block| block["text"].as_str()).collect();
            Some(texts.join("\n"))
        }
        _ => None,
    }
}

fn answers_pick_tier(result: &Value, tiers: &[Tier]) -> bool {
    let Some(questions) = result["questions"].as_array() else {
        return false;
    };
    questions.iter().any(|question| {
        let Some(answer) = question["question"].as_str().and_then(|key| result["answers"][key].as_str()) else {
            return false;
        };
        let Some(options) = question["options"].as_array() else {
            return false;
        };
        if question["multiSelect"] == true {
            return false;
        }
        let tier_for_label = |label: &str| {
            tiers
                .iter()
                .filter(|tier| tier.starts_label(label))
                .max_by_key(|tier| tier.name.len())
        };
        let mut labels = options.iter().filter_map(|option| option["label"].as_str());
        let offered: Vec<_> = labels.clone().filter_map(tier_for_label).map(|tier| tier.name.as_str()).collect();
        if !tiers.iter().all(|tier| offered.contains(&tier.name.as_str())) {
            return false;
        }
        let typed_name = tiers.iter().any(|tier| tier.name.to_lowercase() == answer.trim().to_lowercase());
        typed_name || (labels.any(|label| label == answer) && tier_for_label(answer).is_some())
    })
}
