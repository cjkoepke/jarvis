use crate::git::outgoing::PushedRef;
use crate::paths::Env;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

/// The pushes a review refused, each with the tip its remote ref had at the time.
#[derive(Serialize, Deserialize, Default, Clone, PartialEq)]
#[serde(transparent)]
pub struct RefusedPushes {
    remote_tip_by_ref: BTreeMap<String, String>,
}

impl RefusedPushes {
    /// A missing or unreadable file means no push was refused.
    pub fn load(env: &Env) -> RefusedPushes {
        let text = fs::read_to_string(env.refused_pushes_file()).unwrap_or_default();
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self, env: &Env) -> Result<()> {
        let text = serde_json::to_string_pretty(self)? + "\n";
        fs::write(env.refused_pushes_file(), text).context("recording the refused push")
    }

    /// Whether this ref was refused and its remote has not moved since.
    pub fn holds(&self, remote_url: &str, pushed: &PushedRef) -> bool {
        self.remote_tip_by_ref.get(&key(remote_url, pushed)) == Some(&pushed.remote_sha)
    }

    pub fn remember(&mut self, remote_url: &str, pushed: &PushedRef) {
        self.remote_tip_by_ref.insert(key(remote_url, pushed), pushed.remote_sha.clone());
    }

    pub fn forget(&mut self, remote_url: &str, pushed: &PushedRef) {
        self.remote_tip_by_ref.remove(&key(remote_url, pushed));
    }
}

fn key(remote_url: &str, pushed: &PushedRef) -> String {
    format!("{remote_url} {}", pushed.remote_ref)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pushed_to_main(remote_sha: &str) -> PushedRef {
        PushedRef {
            local_sha: "local".to_string(),
            remote_ref: "refs/heads/main".to_string(),
            remote_sha: remote_sha.to_string(),
        }
    }

    #[test]
    fn a_refusal_holds_only_for_the_same_remote_and_until_its_ref_moves() {
        let mut refused = RefusedPushes::default();
        refused.remember("origin-url", &pushed_to_main("tip"));

        assert!(refused.holds("origin-url", &pushed_to_main("tip")));
        assert!(!refused.holds("origin-url", &pushed_to_main("newer-tip")));
        assert!(!refused.holds("other-url", &pushed_to_main("tip")));

        refused.forget("origin-url", &pushed_to_main("tip"));
        assert!(!refused.holds("origin-url", &pushed_to_main("tip")));
    }
}
