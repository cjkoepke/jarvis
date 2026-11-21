use crate::agents::claude::Claude;
use crate::agents::hooks::{Provider, Trigger};

pub const PROVIDER: Provider = Provider {
    agent: &Claude,
    triggers: &[
        Trigger {
            event: "SessionStart",
            matcher: None,
        },
        Trigger {
            event: "SubagentStart",
            matcher: None,
        },
    ],
    reply: super::the_core,
};
