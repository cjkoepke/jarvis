mod claude;

use super::Hook;

/// Holds back the tools that start agents until the owner has picked an orchestration tier.
pub static HOOK: Hook = Hook {
    name: "launch gate",
    providers: &[claude::PROVIDER],
};
