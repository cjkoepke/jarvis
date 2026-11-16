mod cargo;
mod homebrew;

use anyhow::Result;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The tool that put this binary on the machine, which is also the one that brings it up
/// to date.
pub enum Origin {
    Homebrew(homebrew::Keg),
    Cargo(cargo::Tracked),
}

impl Origin {
    /// None when neither tool keeps a record of the binary.
    pub fn of(executable: &Path) -> Option<Origin> {
        let real = executable.canonicalize().ok()?;
        let from_homebrew = homebrew::Keg::holding(&real).map(Origin::Homebrew);
        from_homebrew.or_else(|| cargo::Tracked::at(&real).map(Origin::Cargo))
    }

    /// The command that installs the newest version in place of this one.
    pub fn upgrade(&self, home: &Path) -> Result<Command> {
        match self {
            Origin::Homebrew(keg) => keg.upgrade(),
            Origin::Cargo(tracked) => tracked.upgrade(home),
        }
    }
}

impl Origin {
    /// What the upgrade did, which for a checkout is less than fetching a newer version.
    pub fn outcome(&self) -> String {
        match self {
            Origin::Cargo(tracked) => tracked.outcome(),
            Origin::Homebrew(_) => "Homebrew has the newest jarvis in place".to_string(),
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Origin::Homebrew(_) => write!(formatter, "Homebrew"),
            Origin::Cargo(_) => write!(formatter, "cargo"),
        }
    }
}

/// The path that still runs jarvis after its next upgrade, which is the one to write into
/// hooks. Homebrew deletes the folder of the version it replaces.
pub fn lasting_executable(started_as: &Path) -> PathBuf {
    let keg = started_as.canonicalize().ok().and_then(|real| homebrew::Keg::holding(&real));
    let lasting = keg.and_then(|keg| keg.lasting_executable());
    lasting.unwrap_or_else(|| started_as.to_path_buf())
}
