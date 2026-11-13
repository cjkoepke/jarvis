use super::Tone;
use anstream::{eprintln, println};

const SUBJECT_WIDTH: usize = 10;

pub fn done(subject: &str, detail: &str) {
    let subject = Tone::Strong.paint(&format!("{subject:<SUBJECT_WIDTH$}"));
    println!("{} {subject} {detail}", Tone::Good.paint("✓"));
}

pub fn note(text: &str) {
    let indent = " ".repeat(SUBJECT_WIDTH + 2);
    println!("{indent} {} {}", Tone::Caution.paint("!"), Tone::Muted.paint(text));
}

pub fn summary(tone: Tone, text: &str) {
    println!("\n{}", tone.paint(text));
}

pub fn error(text: &str) {
    eprintln!("{} {text}", Tone::Bad.paint("error:"));
}

/// @agent The review prints on stderr between git's own lines, so every line
/// carries the `jarvis` prefix to show where it came from.
pub fn review_findings(text: &str) {
    for line in text.lines() {
        eprintln!("{} {line}", Tone::Muted.paint("jarvis"));
    }
}

pub fn review_verdict(tone: Tone, text: &str) {
    eprintln!("{} {}", Tone::Muted.paint("jarvis"), tone.paint(text));
}
