use super::{whitespace, ChangedFile, Finding, Kind, Rule};

enum Location {
    WholeFile,
    Line(usize),
}

/// @agent Returns at most one finding per rule and file, at the first offending
/// line, so a noisy file cannot bury the rest of the review output.
pub fn evaluate(rules: &[Rule], files: &[ChangedFile]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in files {
        for rule in rules.iter().filter(|rule| rule.applies_to(&file.path)) {
            let Some(location) = violation(rule, file) else { continue };
            findings.push(Finding {
                rule_id: rule.id.clone(),
                level: rule.level,
                file: file.path.clone(),
                line: match location {
                    Location::WholeFile => None,
                    Location::Line(number) => Some(number),
                },
                message: rule.message.clone(),
            });
        }
    }
    findings
}

fn violation(rule: &Rule, file: &ChangedFile) -> Option<Location> {
    match rule.kind {
        Kind::ForbiddenPath => rule.pattern_matches(&file.path).then_some(Location::WholeFile),
        Kind::ForbiddenContent => file.first_added_line_where(|line| rule.pattern_matches(line)).map(Location::Line),
        Kind::TrailingWhitespace => file.first_added_line_where(whitespace::has_trailing).map(Location::Line),
        Kind::FinalNewline => {
            let text = file.contents.as_deref()?;
            (!whitespace::ends_with_single_newline(text)).then_some(Location::WholeFile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{changed_file, load::parse};

    #[test]
    fn content_rules_ignore_lines_the_push_did_not_add() {
        let no_debug = [parse("id='dbg'\ndescription=''\nlevel='report'\nkind='forbidden-content'\npattern='dbg!'\nmessage=''").unwrap()];

        let debug_on_untouched_line = changed_file("dbg!(1);\nlet x = 1;\n", &[2]);
        assert!(evaluate(&no_debug, &[debug_on_untouched_line]).is_empty());

        let debug_on_added_line = changed_file("let x = 1;\ndbg!(x);\n", &[2]);
        assert_eq!(evaluate(&no_debug, &[debug_on_added_line])[0].line, Some(2));
    }
}
