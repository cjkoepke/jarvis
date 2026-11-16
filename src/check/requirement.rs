use super::Check;
use anyhow::Result;

/// Something a command needs to be in place: the check on it and, where jarvis can put it
/// right itself, the remedy.
pub struct Requirement<'a> {
    pub check: Check,
    pub remedy: Option<Remedy<'a>>,
}

pub struct Remedy<'a> {
    /// Whose settings change, as `message::done` names it.
    pub subject: &'static str,
    pub summary: String,
    pub grants_trust: bool,
    pub change: Box<dyn FnOnce() -> Result<()> + 'a>,
}

impl Remedy<'_> {
    pub fn apply(self) -> Result<()> {
        (self.change)()
    }
}

impl Check {
    pub fn remedied_by(self, remedy: Remedy) -> Requirement {
        Requirement {
            check: self,
            remedy: Some(remedy),
        }
    }
}

impl From<Check> for Requirement<'_> {
    fn from(check: Check) -> Self {
        Requirement { check, remedy: None }
    }
}
