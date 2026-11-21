mod claude;
mod codex;

use super::{Hook, Reply};
use crate::standards::Standards;
use anyhow::Result;

/// Hands a session or a sub-agent the standards core as it starts.
pub static HOOK: Hook = Hook {
    name: "core delivery",
    providers: &[claude::PROVIDER, codex::PROVIDER],
};

fn the_core(standards: &Standards) -> Result<Reply> {
    Ok(Reply::Context(standards.core.clone()))
}
