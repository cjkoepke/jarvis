use crate::agents;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

const DEFAULT_REVIEW_AGENT: &str = "claude";

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub review: ReviewSettings,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSettings {
    #[serde(default = "default_review_agent")]
    pub agent: String,
    #[serde(default)]
    pub model: Option<String>,
}

impl Default for ReviewSettings {
    fn default() -> ReviewSettings {
        ReviewSettings {
            agent: default_review_agent(),
            model: None,
        }
    }
}

fn default_review_agent() -> String {
    DEFAULT_REVIEW_AGENT.to_string()
}

impl Settings {
    /// A missing file means every setting takes its default.
    pub fn load(file: &Path) -> Result<Settings> {
        if !file.exists() {
            return Ok(Settings::default());
        }
        let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        let settings: Settings = toml::from_str(&text).with_context(|| format!("{} is not valid", file.display()))?;
        agents::chosen(std::slice::from_ref(&settings.review.agent))
            .with_context(|| format!("`review.agent` in {}", file.display()))?;
        Ok(settings)
    }
}
