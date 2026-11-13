use super::{Kind, Level, Rule};
use crate::files::{has_extension, sorted_entries};
use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleFile {
    id: String,
    #[allow(dead_code)]
    description: String,
    level: Level,
    kind: Kind,
    pattern: Option<String>,
    paths: Option<String>,
    message: String,
}

pub fn load_dir(dir: &Path) -> Result<Vec<Rule>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut rules: Vec<Rule> = Vec::new();
    for file in sorted_entries(dir)?.into_iter().filter(|path| has_extension(path, "toml")) {
        let text = std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        let rule = parse(&text).with_context(|| format!("in rule file {}", file.display()))?;
        if rules.iter().any(|existing| existing.id == rule.id) {
            bail!("rule id `{}` is used twice (second time in {})", rule.id, file.display());
        }
        rules.push(rule);
    }
    Ok(rules)
}

pub(super) fn parse(toml_text: &str) -> Result<Rule> {
    let file: RuleFile = toml::from_str(toml_text)?;
    if file.level == Level::Fix && !file.kind.is_fixable() {
        bail!("level `fix` needs a kind the review can fix (final-newline or trailing-whitespace)");
    }
    if !file.kind.is_fixable() && file.pattern.is_none() {
        bail!("this kind needs a `pattern`");
    }
    Ok(Rule {
        pattern: compile(file.pattern, "pattern")?,
        only_paths: compile(file.paths, "paths")?,
        id: file.id,
        level: file.level,
        kind: file.kind,
        message: file.message,
    })
}

fn compile(source: Option<String>, field: &str) -> Result<Option<Regex>> {
    source
        .map(|source| Regex::new(&source).with_context(|| format!("`{field}` is not a valid regex")))
        .transpose()
}
