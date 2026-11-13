use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

const OPEN: &str = "{{";
const CLOSE: &str = "}}";

struct Spec {
    file: &'static str,
    variables: &'static [&'static str],
}

const REVIEW: Spec = Spec {
    file: "review.md",
    variables: &["core", "skills", "commits", "diff", "pass", "fail"],
};
pub struct Prompts {
    pub review: Prompt,
}

impl Prompts {
    pub fn load(dir: &Path) -> Result<Prompts> {
        Ok(Prompts {
            review: Prompt::load(dir, &REVIEW)?,
        })
    }
}

pub struct Prompt {
    template: String,
}

impl Prompt {
    fn load(dir: &Path, spec: &Spec) -> Result<Prompt> {
        let file = dir.join(spec.file);
        let template = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
        if let Some(unknown) = unknown_variable(&template, spec.variables) {
            bail!(
                "{} uses `{OPEN}{unknown}{CLOSE}`; it can only use {}",
                file.display(),
                spec.variables.join(", ")
            );
        }
        Ok(Prompt { template })
    }

    /// Fills every placeholder in one pass, so a value that itself holds `{{` stays as written.
    pub fn render(&self, values: &[(&str, &str)]) -> String {
        let mut rendered = String::with_capacity(self.template.len());
        for segment in segments(&self.template) {
            match segment {
                Segment::Text(text) => rendered.push_str(text),
                Segment::Variable(name) => match values.iter().find(|(key, _)| *key == name) {
                    Some((_, value)) => rendered.push_str(value),
                    None => rendered.push_str(&format!("{OPEN}{name}{CLOSE}")),
                },
            }
        }
        rendered
    }
}

enum Segment<'a> {
    Text(&'a str),
    Variable(&'a str),
}

fn segments(template: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once(OPEN) {
        let Some((name, after_close)) = after_open.split_once(CLOSE) else {
            break;
        };
        segments.push(Segment::Text(before));
        segments.push(Segment::Variable(name.trim()));
        rest = after_close;
    }
    segments.push(Segment::Text(rest));
    segments
}

fn unknown_variable<'a>(template: &'a str, known: &[&str]) -> Option<&'a str> {
    segments(template).into_iter().find_map(|segment| match segment {
        Segment::Variable(name) if !known.contains(&name) => Some(name),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_are_filled_once_and_unknown_ones_are_caught() {
        let prompt = Prompt {
            template: "a {{x}} b {{ y }}".to_string(),
        };
        assert_eq!(prompt.render(&[("x", "{{y}}"), ("y", "2")]), "a {{y}} b 2");
        assert_eq!(unknown_variable("{{x}} {{z}}", &["x"]), Some("z"));
        assert_eq!(unknown_variable("{{x}}", &["x"]), None);
    }
}
