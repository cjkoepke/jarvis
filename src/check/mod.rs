pub mod json;
pub mod table;

use crate::ui::Tone;
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Ok => "✓ ok",
            Status::Warn => "! warn",
            Status::Fail => "✗ fail",
        }
    }

    fn tone(self) -> Tone {
        match self {
            Status::Ok => Tone::Good,
            Status::Warn => Tone::Caution,
            Status::Fail => Tone::Bad,
        }
    }
}

#[derive(Serialize)]
pub struct Check {
    pub area: String,
    pub name: String,
    pub status: Status,
    pub detail: String,
}

pub struct CheckSubject<'a> {
    pub area: &'a str,
    pub name: &'a str,
}

impl CheckSubject<'_> {
    pub fn ok(&self, detail: impl Into<String>) -> Check {
        self.with(Status::Ok, detail)
    }

    pub fn warn(&self, detail: impl Into<String>) -> Check {
        self.with(Status::Warn, detail)
    }

    pub fn fail(&self, detail: impl Into<String>) -> Check {
        self.with(Status::Fail, detail)
    }

    fn with(&self, status: Status, detail: impl Into<String>) -> Check {
        Check {
            area: self.area.to_string(),
            name: self.name.to_string(),
            status,
            detail: detail.into(),
        }
    }
}

pub struct Tally {
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
}

pub fn tally(checks: &[Check]) -> Tally {
    let count = |status: Status| checks.iter().filter(|check| check.status == status).count();
    Tally {
        passed: count(Status::Ok),
        warnings: count(Status::Warn),
        failed: count(Status::Fail),
    }
}

pub fn is_healthy(checks: &[Check]) -> bool {
    tally(checks).failed == 0
}
