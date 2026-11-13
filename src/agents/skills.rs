use super::Agent;
use crate::files::{copy_dir, sorted_entries};
use crate::paths::Env;
use crate::standards::Standards;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// @agent Only skill folders carrying this file are ever replaced or removed. That
/// is what keeps a person's own skill with the same name safe.
const INSTALLED_BY_JARVIS: &str = ".jarvis";

/// Returns one message per skill that was skipped.
pub fn install(agent: &dyn Agent, env: &Env, standards: &Standards) -> Result<Vec<String>> {
    let skills_dir = agent.skills_dir(env);
    let mut skipped = Vec::new();
    for skill in &standards.skills {
        let destination = skills_dir.join(&skill.name);
        if destination.exists() && !is_installed_by_jarvis(&destination) {
            skipped.push(format!(
                "skipped skill `{}`: {} exists and was not installed by jarvis",
                skill.name,
                destination.display()
            ));
            continue;
        }
        if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        copy_dir(&skill.dir, &destination)?;
        fs::write(
            destination.join(INSTALLED_BY_JARVIS),
            "Installed by jarvis. Changes here are overwritten by `jarvis install`.\n",
        )?;
    }
    Ok(skipped)
}

pub fn remove(agent: &dyn Agent, env: &Env) -> Result<()> {
    let skills_dir = agent.skills_dir(env);
    if !skills_dir.is_dir() {
        return Ok(());
    }
    for skill_dir in sorted_entries(&skills_dir)?.into_iter().filter(|dir| is_installed_by_jarvis(dir)) {
        fs::remove_dir_all(&skill_dir)?;
    }
    Ok(())
}

pub fn not_installed<'a>(agent: &dyn Agent, env: &Env, standards: &'a Standards) -> Vec<&'a str> {
    let skills_dir = agent.skills_dir(env);
    let skill_names = standards.skills.iter().map(|skill| skill.name.as_str());
    skill_names.filter(|name| !is_installed_by_jarvis(&skills_dir.join(name))).collect()
}

fn is_installed_by_jarvis(skill_dir: &Path) -> bool {
    skill_dir.join(INSTALLED_BY_JARVIS).exists()
}
