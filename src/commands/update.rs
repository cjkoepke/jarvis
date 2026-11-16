use crate::origin::Origin;
use crate::paths::Env;
use crate::ui::message;
use anyhow::{bail, Context, Result};
use std::process::{Command, ExitCode};

pub fn run(env: &Env) -> Result<ExitCode> {
    let Some(origin) = Origin::of(&env.jarvis_executable) else {
        bail!(
            "neither Homebrew nor cargo installed {}, so jarvis cannot update it; replace it yourself, then run `jarvis install`",
            env.jarvis_executable.display()
        );
    };
    let upgraded = origin.upgrade(&env.home)?.status().with_context(|| format!("starting {origin}"))?;
    if !upgraded.success() {
        bail!("{origin} did not finish the update, so nothing else was changed");
    }
    message::done("update", &origin.outcome());

    let installed = Command::new(&env.jarvis_executable)
        .arg("--home")
        .arg(&env.home)
        .arg("install")
        .status()
        .with_context(|| format!("starting the updated {}", env.jarvis_executable.display()))?;
    Ok(if installed.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
