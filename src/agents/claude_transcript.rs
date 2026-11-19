use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// What the owner has said in a Claude Code session since the last prompt they typed.
#[derive(Default)]
pub struct OwnerTurn {
    pub prompt: Option<String>,
    /// Their answers to the questions Claude asked since, with the AskUserQuestion tool.
    pub answers: Vec<String>,
}

/// Reads the session transcript, one JSON entry per line. Background task notifications and
/// messages from other agents are entries too, but only a typed prompt starts a new turn.
pub fn owner_turn(transcript: &Path) -> Result<OwnerTurn> {
    let file = File::open(transcript).with_context(|| format!("reading {}", transcript.display()))?;
    let mut turn = OwnerTurn::default();
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
        if let Some(prompt) = typed_prompt(&entry) {
            turn = OwnerTurn {
                prompt: Some(prompt),
                answers: Vec::new(),
            };
        } else {
            turn.answers.extend(answers(&entry));
        }
    }
    Ok(turn)
}

/// Transcripts reach hundreds of megabytes, so only lines that can hold a prompt or an answer are parsed.
fn may_be_from_the_owner(line: &str) -> bool {
    line.contains("\"human\"") || line.contains("\"answers\"")
}

fn typed_prompt(entry: &Value) -> Option<String> {
    if entry["origin"]["kind"] != "human" {
        return None;
    }
    match &entry["message"]["content"] {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            let texts: Vec<&str> = blocks.iter().filter_map(|block| block["text"].as_str()).collect();
            Some(texts.join("\n"))
        }
        _ => None,
    }
}

fn answers(entry: &Value) -> Vec<String> {
    let result = &entry["toolUseResult"];
    if result["questions"].is_null() {
        return Vec::new();
    }
    let answers = result["answers"].as_object().into_iter().flat_map(|answers| answers.values());
    answers.filter_map(Value::as_str).map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const PROMPT: &str = r#"{"type":"user","origin":{"kind":"human"},"message":{"role":"user","content":"PROMPT"}}"#;
    const ANSWER: &str = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t1","content":"answered"}]},"toolUseResult":{"questions":[{"question":"Which tier?"}],"answers":{"Which tier?":"ANSWER"}}}"#;
    const NOTIFICATION: &str = r#"{"type":"user","origin":{"kind":"task-notification"},"turnOrigin":"human","message":{"role":"user","content":"<task-notification>done</task-notification>"}}"#;

    fn turn_of(lines: &[String]) -> OwnerTurn {
        let dir = tempfile::tempdir().unwrap();
        let transcript = dir.path().join("session.jsonl");
        fs::write(&transcript, lines.join("\n")).unwrap();
        owner_turn(&transcript).unwrap()
    }

    #[test]
    fn a_typed_prompt_starts_a_turn_and_a_notification_does_not() {
        let turn = turn_of(&[
            PROMPT.replace("PROMPT", "first"),
            ANSWER.replace("ANSWER", "Economy"),
            PROMPT.replace("PROMPT", "second"),
            ANSWER.replace("ANSWER", "Balanced"),
            NOTIFICATION.to_string(),
        ]);
        assert_eq!(turn.prompt.as_deref(), Some("second"));
        assert_eq!(turn.answers, ["Balanced"]);
    }
}
