mod evaluate;
mod fix;
mod load;
mod whitespace;

pub use evaluate::evaluate;
pub use fix::{fix, FileFix};
pub use load::load_dir;

use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Report,
    Fix,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    ForbiddenPath,
    ForbiddenContent,
    FinalNewline,
    TrailingWhitespace,
}

impl Kind {
    fn is_fixable(self) -> bool {
        matches!(self, Kind::FinalNewline | Kind::TrailingWhitespace)
    }
}

pub struct Rule {
    pub id: String,
    pub level: Level,
    pub kind: Kind,
    pub message: String,
    pattern: Option<Regex>,
    only_paths: Option<Regex>,
}

impl Rule {
    fn applies_to(&self, path: &str) -> bool {
        self.only_paths.as_ref().is_none_or(|only_paths| only_paths.is_match(path))
    }

    fn pattern_matches(&self, text: &str) -> bool {
        self.pattern.as_ref().is_some_and(|pattern| pattern.is_match(text))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule_id: String,
    pub level: Level,
    pub file: String,
    pub line: Option<usize>,
    pub message: String,
}

/// @agent Line numbers are 1-based. `contents` is `None` for a binary file, which
/// rules can then only judge by its path.
pub struct ChangedFile {
    pub path: String,
    pub contents: Option<String>,
    pub added_lines: HashSet<usize>,
}

impl ChangedFile {
    fn first_added_line_where(&self, is_violation: impl Fn(&str) -> bool) -> Option<usize> {
        let numbered_lines = self.contents.as_deref()?.lines().enumerate().map(|(index, line)| (index + 1, line));
        numbered_lines
            .filter(|(number, _)| self.added_lines.contains(number))
            .find(|(_, line)| is_violation(line))
            .map(|(number, _)| number)
    }
}

#[cfg(test)]
fn changed_file(contents: &str, added_lines: &[usize]) -> ChangedFile {
    ChangedFile {
        path: "lib.rs".into(),
        contents: Some(contents.into()),
        added_lines: added_lines.iter().copied().collect(),
    }
}
