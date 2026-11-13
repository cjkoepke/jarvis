use super::verdict::{FAIL, PASS};
use crate::git::outgoing::Outgoing;
use crate::standards::Standards;

pub fn build(standards: &Standards, outgoing: &[Outgoing]) -> String {
    let commit_messages: Vec<&str> = outgoing.iter().map(|pushed| pushed.commit_messages.as_str()).collect();
    let diffs: Vec<&str> = outgoing.iter().map(|pushed| pushed.diff.as_str()).collect();
    format!(
        "\
You are reviewing commits that are about to be pushed. Judge them against the standards below and nothing else.

Review only what the outgoing commits add or change, including their commit messages. Leave alone anything they did not touch.
You are in the repository's working tree and may read any file for context. The diff is what is being pushed; files on disk can also hold uncommitted edits.

# Standards

{core}

# Skills

{skills}

# Outgoing commits

{commit_messages}

# Diff

{diffs}

# Reply

List each violation as `<file>:<line>`, the standard it breaks, and what to change. Report only violations you are sure of. Do not praise, summarise, or suggest improvements the standards do not ask for.
End with exactly one final line: `{PASS}` when there are no violations, `{FAIL}` otherwise.
",
        core = standards.core,
        skills = skills_section(standards),
        commit_messages = commit_messages.join("\n\n"),
        diffs = diffs.join("\n"),
    )
}

fn skills_section(standards: &Standards) -> String {
    if standards.skills.is_empty() {
        return "None.".to_string();
    }
    let mut section =
        "Each skill is a longer procedure. Before judging, read the file of every skill that bears on these commits.\n".to_string();
    for skill in &standards.skills {
        section.push_str(&format!("\n- {}: {} ({})", skill.name, skill.description, skill.file().display()));
    }
    section
}
