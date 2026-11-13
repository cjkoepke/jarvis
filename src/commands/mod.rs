pub mod doctor;
pub mod hook;
pub mod install;
pub mod pre_push;
pub mod uninstall;

use crate::paths::Env;
use anyhow::{bail, Result};

fn require_flag_for_real_home(env: &Env, real_flag: bool) -> Result<()> {
    if env.is_real_home() && !real_flag {
        bail!(
            "{} is your real home directory. Pass --real to change your agent and git configuration there, or --home <dir> to try jarvis in a throwaway directory.",
            env.home.display()
        );
    }
    Ok(())
}
