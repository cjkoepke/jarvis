use anyhow::{bail, Result};
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CELLAR: &str = "Cellar";
const RECEIPT: &str = "INSTALL_RECEIPT.json";
const CORE_TAP: &str = "homebrew/core";

/// The folder Homebrew installed one version of a formula into.
pub struct Keg {
    homebrew: PathBuf,
    formula: String,
    tap: Option<String>,
    binary: OsString,
}

impl Keg {
    /// Homebrew keeps a binary at `Cellar/<formula>/<version>/bin/`, with a receipt in the
    /// version's folder.
    pub fn holding(real_executable: &Path) -> Option<Keg> {
        let bin = real_executable.parent().filter(|bin| bin.ends_with("bin"))?;
        let keg = bin.parent()?;
        let rack = keg.parent()?;
        let cellar = rack.parent().filter(|cellar| cellar.ends_with(CELLAR))?;

        let receipt: Value = serde_json::from_str(&fs::read_to_string(keg.join(RECEIPT)).ok()?).ok()?;
        let tap = receipt.get("source").and_then(|source| source.get("tap")).and_then(Value::as_str);
        Some(Keg {
            homebrew: cellar.parent()?.to_path_buf(),
            formula: rack.file_name()?.to_str()?.to_string(),
            tap: tap.map(str::to_string),
            binary: real_executable.file_name()?.to_owned(),
        })
    }

    pub fn upgrade(&self) -> Result<Command> {
        let brew = self.homebrew.join("bin/brew");
        if !brew.is_file() {
            bail!("Homebrew installed this jarvis, but its `brew` is not at {}", brew.display());
        }
        let mut upgrade = Command::new(brew);
        upgrade.args(["upgrade", &self.formula_with_its_tap()]);
        Ok(upgrade)
    }

    /// Homebrew points `opt/<formula>` at each new version.
    pub fn lasting_executable(&self) -> Option<PathBuf> {
        let lasting = self.homebrew.join("opt").join(&self.formula).join("bin").join(&self.binary);
        lasting.is_file().then_some(lasting)
    }

    /// Named with its tap, Homebrew looks for a newer version there before it answers.
    fn formula_with_its_tap(&self) -> String {
        match &self.tap {
            Some(tap) if tap != CORE_TAP => format!("{tap}/{}", self.formula),
            _ => self.formula.clone(),
        }
    }
}
