use std::collections::HashSet;

const TRAILING: [char; 2] = [' ', '\t'];

pub fn has_trailing(line: &str) -> bool {
    line.ends_with(TRAILING)
}

pub fn strip_trailing_on(text: &str, line_numbers: &HashSet<usize>) -> String {
    let mut stripped = String::with_capacity(text.len());
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let (body, line_ending) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        let line_number = index + 1;
        stripped.push_str(if line_numbers.contains(&line_number) {
            body.trim_end_matches(TRAILING)
        } else {
            body
        });
        stripped.push_str(line_ending);
    }
    stripped
}

pub fn ends_with_single_newline(text: &str) -> bool {
    text.is_empty() || (text.ends_with('\n') && !text.ends_with("\n\n"))
}

pub fn with_single_final_newline(text: &str) -> String {
    let body = text.trim_end_matches('\n');
    if body.is_empty() {
        return String::new();
    }
    format!("{body}\n")
}
