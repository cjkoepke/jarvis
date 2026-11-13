use crate::files::{has_extension, sorted_entries};
use crate::rules::{self, Rule};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// @agent Claude Code drops hook context longer than 10,000 characters
/// (https://code.claude.com/docs/en/hooks). The cap sits well below that because a
/// core this long stops being followed.
pub const CORE_MAX_CHARS: usize = 6000;

pub struct Skill {
    pub name: String,
    pub dir: PathBuf,
}

pub struct Standards {
    pub core: String,
    pub skills: Vec<Skill>,
    pub rules: Vec<Rule>,
}

impl Standards {
    pub fn load(root: &Path) -> Result<Standards> {
        Ok(Standards {
            core: load_core(&root.join("core"))?,
            skills: load_skills(&root.join("skills"))?,
            rules: rules::load_dir(&root.join("rules"))?,
        })
    }

    /// @agent The header is a statement of fact on purpose. Claude Code's hook docs
    /// (https://code.claude.com/docs/en/hooks) warn that injected text phrased as
    /// commands can be treated as a prompt injection.
    pub fn session_context(&self) -> String {
        let mut context = format!("Working standards for this machine, delivered by jarvis.\n\n{}", self.core);
        if !self.skills.is_empty() {
            let skill_names: Vec<&str> = self.skills.iter().map(|skill| skill.name.as_str()).collect();
            context.push_str(&format!(
                "\n\nLonger procedures are installed as skills: {}.",
                skill_names.join(", ")
            ));
        }
        context
    }

    pub fn subagent_context(&self) -> String {
        format!(
            "Working standards for this machine, delivered by jarvis. They apply to delegated work as well.\n\n{}",
            self.core
        )
    }
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
    let skill_dirs = sorted_entries(dir)?.into_iter().filter(|path| path.join("SKILL.md").is_file());
    Ok(skill_dirs
        .map(|dir| Skill {
            name: dir.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            dir,
        })
        .collect())
}
