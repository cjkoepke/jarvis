mod audit;
mod init;

use crate::agents::{self, Agent};
use crate::check::requirement::{Remedy, Requirement};
use crate::check::CheckSubject;
use crate::cli::RepoCmd;
use crate::git::{self, repo::Repo};
use crate::paths::Env;
use crate::record::InstallRecord;
use crate::settings::Settings;
use crate::ui::message;
use anyhow::{bail, Result};
use std::process::ExitCode;

const FIXING: CheckSubject = CheckSubject {
    area: "Review",
    name: "fixing",
};

pub fn run(env: &Env, command: RepoCmd) -> Result<ExitCode> {
    match command {
        RepoCmd::Init { yes } => init::run(env, yes),
        RepoCmd::Audit { format } => audit::run(env, format),
    }
}

fn checkout_run_in() -> Result<Repo> {
    let is_inside_a_checkout = git::run(&["rev-parse", "--is-inside-work-tree"]).is_ok_and(|inside| inside == "true");
    if !is_inside_a_checkout {
        bail!("`jarvis repo` works in the checkout of a git repository; run it there");
    }
    Repo::current()
}

/// What has to be in place before the reviewer fixes what it finds in `repo`.
fn requirements<'a>(env: &'a Env, repo: &'a Repo) -> Vec<Requirement<'a>> {
    let mut requirements = Vec::new();
    for agent in agents(env) {
        requirements.extend(agent.fixing_requirements(env, repo));
    }
    requirements.push(fixing_setting_requirement(env));
    requirements
}

/// The agents jarvis is installed for, and the one that reviews when it is not among them.
fn agents(env: &Env) -> Vec<&'static dyn Agent> {
    let installed = InstallRecord::load(env).map(|record| record.agent_ids).unwrap_or_default();
    let reviewer = Settings::load(&env.settings_file()).map(|settings| settings.review.agent).ok();

    let mut ids = installed;
    ids.extend(reviewer.filter(|reviewer| !ids.contains(reviewer)));
    ids.iter().filter_map(|id| agents::by_id(id)).collect()
}

fn fixing_setting_requirement(env: &Env) -> Requirement<'_> {
    let file = env.settings_file();
    match Settings::load(&file) {
        Ok(settings) if settings.review.fix => FIXING.ok("`review.fix` is on").into(),
        Ok(mut settings) => FIXING
            .warn("`review.fix` is off, so the reviewer only reports what it finds; switch it on with `jarvis settings`")
            .remedied_by(Remedy {
                subject: "settings",
                summary: format!("`review.fix` switched on in {}", file.display()),
                grants_trust: false,
                change: Box::new(move || {
                    settings.review.fix = true;
                    settings.save(&file)
                }),
            }),
        Err(error) => FIXING.warn(format!("{error:#}, so pushes go out unreviewed")).into(),
    }
}

fn apply(remedy: Remedy) -> Result<()> {
    let subject = remedy.subject;
    let summary = remedy.summary.clone();
    remedy.apply()?;
    message::done(subject, &summary);
    Ok(())
}
