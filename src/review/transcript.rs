use super::fixes::Fixing;
use crate::files;
use crate::paths::Env;
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const KEPT_REVIEWS: usize = 20;

/// The folder one review leaves behind in `~/.jarvis/reviews`.
pub struct Transcript {
    dir: PathBuf,
}

#[derive(Serialize)]
pub struct Subject<'a> {
    pub repository: &'a Path,
    pub remote: Option<&'a str>,
    pub refs: &'a [String],
    pub commits: &'a [String],
    pub agent: &'a str,
    pub model: Option<&'a str>,
    pub fixing: Fixing,
}

impl Transcript {
    /// Folders are named by the time their review started, so sorting them by name
    /// puts the oldest first.
    pub fn start(env: &Env) -> Result<Transcript> {
        let reviews = env.reviews_dir();
        fs::create_dir_all(&reviews).with_context(|| format!("creating {}", reviews.display()))?;
        let started = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let dir = tempfile::Builder::new()
            .prefix(&format!("{}-{:09}-", started.as_secs(), started.subsec_nanos()))
            .tempdir_in(&reviews)
            .with_context(|| format!("creating a folder in {}", reviews.display()))?
            .keep();
        drop_all_but_newest(&reviews, &dir, KEPT_REVIEWS).ok();
        Ok(Transcript { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn prompt_file(&self) -> PathBuf {
        self.dir.join("prompt.md")
    }

    pub fn reply_file(&self) -> PathBuf {
        self.dir.join("reply.md")
    }

    /// Everything the agent printed besides its reply. Codex prints its whole run there.
    pub fn errors_file(&self) -> PathBuf {
        self.dir.join("errors.log")
    }

    pub fn record(&self, subject: &Subject) -> Result<()> {
        let text = serde_json::to_string_pretty(subject)? + "\n";
        fs::write(self.dir.join("review.json"), text).context("recording what is reviewed")
    }
}

fn drop_all_but_newest(reviews: &Path, current: &Path, kept: usize) -> Result<()> {
    let mut oldest_first = files::sorted_entries(reviews)?;
    oldest_first.retain(|review| review.is_dir() && review != current);
    let outdated = oldest_first.len().saturating_sub(kept.saturating_sub(1));
    for review in &oldest_first[..outdated] {
        fs::remove_dir_all(review)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_newest_reviews_are_kept() {
        let reviews = tempfile::tempdir().unwrap();
        let stray_file = reviews.path().join(".DS_Store");
        fs::write(&stray_file, "").unwrap();
        for review in ["1700000300-c", "1700000100-a", "1700000200-b"] {
            fs::create_dir(reviews.path().join(review)).unwrap();
            fs::write(reviews.path().join(review).join("reply.md"), "VERDICT: PASS").unwrap();
        }

        let current = reviews.path().join("1700000300-c");
        drop_all_but_newest(reviews.path(), &current, 2).unwrap();

        let kept = files::sorted_entries(reviews.path()).unwrap();
        let newest_two = ["1700000200-b", "1700000300-c"].map(|review| reviews.path().join(review));
        assert_eq!(kept, [&[stray_file][..], &newest_two[..]].concat());
    }

    #[test]
    fn retention_never_removes_the_review_being_started() {
        let reviews = tempfile::tempdir().unwrap();
        let current = reviews.path().join("1700000300-a");
        fs::create_dir(&current).unwrap();
        fs::create_dir(reviews.path().join("1700000300-z")).unwrap();

        drop_all_but_newest(reviews.path(), &current, 1).unwrap();

        assert_eq!(files::sorted_entries(reviews.path()).unwrap(), vec![current]);
    }
}
