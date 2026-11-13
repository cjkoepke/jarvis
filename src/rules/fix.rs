use super::{whitespace, ChangedFile, Kind, Level, Rule};

pub struct FileFix {
    pub path: String,
    pub contents: String,
}

/// @agent Whitespace is only stripped on lines the push added, so a fix never
/// rewrites code the author did not touch.
pub fn fix(rules: &[Rule], file: &ChangedFile) -> Option<FileFix> {
    let original = file.contents.as_deref()?;
    let mut fixed = original.to_string();
    for rule in rules.iter().filter(|rule| rule.level == Level::Fix && rule.applies_to(&file.path)) {
        fixed = match rule.kind {
            Kind::TrailingWhitespace => whitespace::strip_trailing_on(&fixed, &file.added_lines),
            Kind::FinalNewline => whitespace::with_single_final_newline(&fixed),
            Kind::ForbiddenPath | Kind::ForbiddenContent => fixed,
        };
    }
    (fixed != original).then(|| FileFix {
        path: file.path.clone(),
        contents: fixed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{changed_file, load::parse};

    #[test]
    fn fixes_touch_only_added_lines_and_the_end_of_the_file() {
        let fix_rules = [
            parse("id='ws'\ndescription=''\nlevel='fix'\nkind='trailing-whitespace'\nmessage=''").unwrap(),
            parse("id='nl'\ndescription=''\nlevel='fix'\nkind='final-newline'\nmessage=''").unwrap(),
        ];

        let second_line_added = changed_file("untouched  \nadded  \nlast", &[2]);
        assert_eq!(fix(&fix_rules, &second_line_added).unwrap().contents, "untouched  \nadded\nlast\n");

        let already_clean = changed_file("clean\n", &[1]);
        assert!(fix(&fix_rules, &already_clean).is_none());
    }
}
