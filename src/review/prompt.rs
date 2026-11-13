use super::verdict::{FAIL, PASS};
use crate::git::outgoing::Outgoing;
use crate::standards::Standards;

pub fn build(standards: &Standards, outgoing: &[Outgoing]) -> String {
    let commits: Vec<&str> = outgoing.iter().map(|pushed| pushed.commit_messages.as_str()).collect();
    let diffs: Vec<&str> = outgoing.iter().map(|pushed| pushed.diff.as_str()).collect();
    let skills: Vec<String> = standards
        .skills
        .iter()
        .map(|skill| format!("- {}: {} ({})", skill.name, skill.description, skill.file().display()))
        .collect();

    standards.prompts.review.render(&[
        ("core", &standards.core),
        ("skills", &skills.join("\n")),
        ("commits", &commits.join("\n\n")),
        ("diff", &diffs.join("\n")),
        ("pass", PASS),
        ("fail", FAIL),
    ])
}
