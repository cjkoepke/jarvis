use super::{is_healthy, json, table, tally, Check};
use crate::cli::Format;
use crate::ui::{message, Tone};
use anyhow::Result;
use std::process::ExitCode;

pub fn print(checks: &[Check], format: Format) -> Result<ExitCode> {
    match format {
        Format::Table => {
            anstream::println!("{}", table::render(checks));
            print_summary(checks);
        }
        Format::Json => println!("{}", json::render(checks)?),
    }
    Ok(if is_healthy(checks) { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

fn print_summary(checks: &[Check]) {
    let tally = tally(checks);
    if tally.failed > 0 {
        let text = format!(
            "✗ {} failed, {} passed. Each failed row says how to fix it.",
            tally.failed, tally.passed
        );
        message::summary(Tone::Bad, &text);
    } else if tally.warnings > 0 {
        message::summary(Tone::Caution, &format!("! {} passed, {} to look at.", tally.passed, tally.warnings));
    } else {
        message::summary(Tone::Good, &format!("✓ All {} checks passed.", tally.passed));
    }
}
