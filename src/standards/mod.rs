pub mod custom;
pub mod source;
pub mod tiers;

use crate::files::{has_extension, sorted_entries};
use anyhow::{bail, Context, Result};
use source::Source;
use std::fs;
use std::path::{Path, PathBuf};

pub const CORE_MAX_CHARS: usize = 6000;

pub(crate) const SKILL_FILE: &str = "SKILL.md";

pub struct Skill {
    pub name: String,
    pub dir: PathBuf,
}

impl Skill {
    pub fn file(&self) -> PathBuf {
        self.dir.join(SKILL_FILE)
    }
}

pub struct Standards {
    pub core: String,
    pub skills: Vec<Skill>,
}

impl Standards {
    pub fn load(root: &Path) -> Result<Standards> {
        Ok(Standards {
            core: load_core(&root.join("core"))?,
            skills: load_skills(&root.join("skills"))?,
        })
    }
}

/// Writes the standards an install delivers into `dir`: the source's, with the person's own
/// from `custom_dir` laid over them.
pub fn assemble(source: &Source, custom_dir: &Path, dir: &Path) -> Result<()> {
    source.copy_into(dir)?;
    custom::lay_over(custom_dir, dir)
}

fn load_core(dir: &Path) -> Result<String> {
    let markdown_files: Vec<PathBuf> = sorted_entries(dir)?.into_iter().filter(|path| has_extension(path, "md")).collect();
    if markdown_files.is_empty() {
        bail!("{} holds no .md file", dir.display());
    }

    let mut sections = Vec::new();
    for file in &markdown_files {
        let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        sections.push(text.trim().to_string());
    }
    let core = sections.join("\n\n");

    let size = core.chars().count();
    if size > CORE_MAX_CHARS {
        bail!("the core in {} is {size} characters; the cap is {CORE_MAX_CHARS}", dir.display());
    }
    Ok(core)
}

fn load_skills(dir: &Path) -> Result<Vec<Skill>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let skill_dirs = sorted_entries(dir)?.into_iter().filter(|path| path.join(SKILL_FILE).is_file());
    Ok(skill_dirs
        .map(|dir| Skill {
            name: dir.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            dir,
        })
        .collect())
}
