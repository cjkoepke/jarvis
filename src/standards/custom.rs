use super::SKILL_FILE;
use crate::files::{copy_dir, has_extension, sorted_entries};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const CORE: &str = "core";
const SKILLS: &str = "skills";

/// The core files and skills in a standards folder, each named by its path in the folder.
/// A piece is what a person takes over as a whole: a core file, or a skill with all its files.
pub fn pieces(standards_dir: &Path) -> Result<Vec<String>> {
    let core_files = entries(&standards_dir.join(CORE))?
        .into_iter()
        .filter(|(_, path)| has_extension(path, "md"))
        .map(|(name, _)| format!("{CORE}/{name}"));
    let skills = entries(&standards_dir.join(SKILLS))?
        .into_iter()
        .filter(|(_, path)| path.join(SKILL_FILE).is_file())
        .map(|(name, _)| format!("{SKILLS}/{name}"));
    Ok(core_files.chain(skills).collect())
}

/// What sits where a piece belongs but is not one, so it is not delivered: a core file
/// that does not end in `.md`, or a skill without its `SKILL.md`.
pub fn unusable(standards_dir: &Path) -> Result<Vec<String>> {
    let usable = pieces(standards_dir)?;
    let mut found = Vec::new();
    for part in [CORE, SKILLS] {
        let names = entries(&standards_dir.join(part))?.into_iter().map(|(name, _)| name);
        let visible = names.filter(|name| !name.starts_with('.'));
        found.extend(visible.map(|name| format!("{part}/{name}")).filter(|piece| !usable.contains(piece)));
    }
    Ok(found)
}

/// Puts each of a person's own pieces in place of the one with its name, or beside the others.
pub fn lay_over(custom_dir: &Path, standards_dir: &Path) -> Result<()> {
    for piece in pieces(custom_dir)? {
        copy_piece(&custom_dir.join(&piece), &standards_dir.join(&piece))?;
    }
    Ok(())
}

pub fn copy_piece(from: &Path, to: &Path) -> Result<()> {
    if to.is_dir() {
        fs::remove_dir_all(to).with_context(|| format!("replacing {}", to.display()))?;
    }
    if from.is_dir() {
        return copy_dir(from, to);
    }
    if let Some(folder) = to.parent() {
        fs::create_dir_all(folder).with_context(|| format!("creating {}", folder.display()))?;
    }
    fs::copy(from, to).with_context(|| format!("copying to {}", to.display()))?;
    Ok(())
}

fn entries(dir: &Path) -> Result<Vec<(String, std::path::PathBuf)>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let named = sorted_entries(dir)?.into_iter().filter_map(|path| {
        let name = path.file_name()?.to_str()?.to_string();
        Some((name, path))
    });
    Ok(named.collect())
}
