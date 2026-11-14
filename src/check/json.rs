use super::{is_healthy, Check};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
struct Report<'a> {
    healthy: bool,
    checks: &'a [Check],
}

pub fn render(checks: &[Check]) -> Result<String> {
    Ok(serde_json::to_string_pretty(&Report {
        healthy: is_healthy(checks),
        checks,
    })?)
}
