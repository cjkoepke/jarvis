const PASS: &str = "VERDICT: PASS";
const FAIL: &str = "VERDICT: FAIL";

#[derive(Debug, PartialEq)]
pub enum Verdict {
    Pass,
    Fail,
}

pub struct Review {
    pub verdict: Verdict,
    pub findings: String,
}

pub fn parse(reply: &str) -> Option<Review> {
    let reply = reply.trim();
    let (findings, last_line) = reply.rsplit_once('\n').unwrap_or(("", reply));
    let verdict = if last_line.contains(PASS) {
        Verdict::Pass
    } else if last_line.contains(FAIL) {
        Verdict::Fail
    } else {
        return None;
    };
    Some(Review {
        verdict,
        findings: findings.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_verdict_is_read_from_the_last_line_only() {
        let failed = parse("src/a.rs:3 explains the code\n\n**VERDICT: FAIL**\n").unwrap();
        assert_eq!(failed.verdict, Verdict::Fail);
        assert_eq!(failed.findings, "src/a.rs:3 explains the code");

        assert_eq!(parse("VERDICT: PASS").unwrap().verdict, Verdict::Pass);
        assert!(parse("VERDICT: PASS\nbut on reflection there is a problem").is_none());
        assert!(parse("").is_none());
    }
}
