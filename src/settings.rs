use crate::agents;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const DEFAULT_REVIEW_AGENT: &str = "claude";

#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub review: ReviewSettings,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReviewSettings {
    #[serde(default = "default_review_agent")]
    pub agent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub fix: bool,
}

impl Default for ReviewSettings {
    fn default() -> ReviewSettings {
        ReviewSettings {
            agent: default_review_agent(),
            model: None,
            fix: false,
        }
    }
}

fn default_review_agent() -> String {
    DEFAULT_REVIEW_AGENT.to_string()
}

impl ReviewSettings {
    pub fn switch_agent(&mut self, agent_id: &str) {
        if self.agent != agent_id {
            self.agent = agent_id.to_string();
            self.model = None;
        }
    }
}

impl Settings {
    /// A missing file means every setting takes its default.
    pub fn load(file: &Path) -> Result<Settings> {
        if !file.exists() {
            return Ok(Settings::default());
        }
        let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let settings: Settings = toml::from_str(&text).with_context(|| format!("{} is not valid", file.display()))?;
        agents::chosen(std::slice::from_ref(&settings.review.agent)).with_context(|| format!("`review.agent` in {}", file.display()))?;
        Ok(settings)
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = toml::to_string(self).context("writing the settings as TOML")?;
        fs::write(file, text).with_context(|| format!("writing {}", file.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_settings_load_back_the_same() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-yet-there/settings.toml");
        let mut settings = Settings::default();
        settings.review.model = Some("fable".to_string());
        settings.save(&file).unwrap();
        assert!(Settings::load(&file).unwrap() == settings);

        settings.review.switch_agent("codex");
        settings.save(&file).unwrap();
        assert!(Settings::load(&file).unwrap() == settings);
        assert!(settings.review.model.is_none());
    }
}
