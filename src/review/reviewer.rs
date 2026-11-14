use crate::agents::{self, Agent};
use crate::files::find_on_path;
use crate::git::outgoing::Outgoing;
use crate::paths::Env;
use crate::standards::{Skill, Standards};
use anyhow::{bail, Context, Result};
use std::fmt;
use std::fs::{self, File};
use std::process::{Child, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

const TIME_LIMIT: Duration = Duration::from_secs(600);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

const REVIEW_SKILL: &str = "review";
const STANDARDS_VARIABLE: &str = "JARVIS_STANDARDS";
const REMOTE_VARIABLE: &str = "JARVIS_PUSH_REMOTE";
const COMMITS_VARIABLE: &str = "JARVIS_PUSH_COMMITS";

pub struct Reviewer<'a> {
    pub agent: &'static dyn Agent,
    pub model: Option<&'a str>,
    core: &'a str,
    skill: &'a Skill,
}

impl fmt::Display for Reviewer<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.model {
            Some(model) => write!(formatter, "{} on {model}", self.agent.display_name()),
            None => write!(formatter, "{} on its default model", self.agent.display_name()),
        }
    }
}

pub fn choose(standards: &Standards) -> Result<Reviewer<'_>> {
    let settings = &standards.settings.review;
    let agent = agents::by_id(&settings.agent).with_context(|| format!("unknown review agent `{}`", settings.agent))?;
    if find_on_path(agent.executable_name()).is_none() {
        bail!("`{}` is not on PATH", agent.executable_name());
    }
    let skill = standards
        .skills
        .iter()
        .find(|skill| skill.name == REVIEW_SKILL)
        .with_context(|| format!("the standards have no `{REVIEW_SKILL}` skill"))?;
    Ok(Reviewer {
        agent,
        model: settings.model.as_deref(),
        core: &standards.core,
        skill,
    })
}

pub fn ask(reviewer: &Reviewer, env: &Env, outgoing: &Outgoing) -> Result<String> {
    let agent = reviewer.agent;
    let scratch = tempfile::tempdir()?;
    let prompt_file = scratch.path().join("prompt.md");
    let reply_file = scratch.path().join("reply.md");
    let errors_file = scratch.path().join("errors.log");
    let skill_file = reviewer.skill.file();
    let skill = fs::read_to_string(&skill_file).with_context(|| format!("reading {}", skill_file.display()))?;
    fs::write(&prompt_file, format!("{}\n\n{skill}", reviewer.core))?;

    let standards_dir = env.installed_standards_dir();
    let mut command = agent.review_command(&standards_dir, reviewer.model);
    command
        .env(STANDARDS_VARIABLE, &standards_dir)
        .env(REMOTE_VARIABLE, &outgoing.remote)
        .env(COMMITS_VARIABLE, outgoing.commits.join(" "))
        .stdin(File::open(&prompt_file)?)
        .stdout(File::create(&reply_file)?)
        .stderr(File::create(&errors_file)?);
    let mut process = command.spawn().with_context(|| format!("starting {}", agent.display_name()))?;

    let status = wait_within_time_limit(&mut process)?;
    if !status.success() {
        let errors = fs::read_to_string(&errors_file).unwrap_or_default();
        let last_error = errors.lines().rev().find(|line| !line.trim().is_empty()).unwrap_or_default();
        bail!("{} stopped with {status}: {last_error}", agent.display_name());
    }
    Ok(fs::read_to_string(&reply_file)?)
}

fn wait_within_time_limit(reviewer: &mut Child) -> Result<ExitStatus> {
    let deadline = Instant::now() + TIME_LIMIT;
    loop {
        if let Some(status) = reviewer.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            reviewer.kill().ok();
            reviewer.wait().ok();
            bail!("the reviewer took longer than {} minutes", TIME_LIMIT.as_secs() / 60);
        }
        thread::sleep(POLL_INTERVAL);
    }
}
