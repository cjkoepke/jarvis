use crate::check::requirement::Requirement;
use crate::check::{self, Check, Status};
use crate::git::repo::Repo;
use crate::paths::Env;
use crate::ui::prompt::{Answer, Picker, Radio, Row, Screen};
use crate::ui::{message, Tone};
use anyhow::{bail, Result};
use std::process::ExitCode;

const ANSWER_HELP: &str = "↑↓ move · enter answer · esc leave";

pub fn run(env: &Env, trust_without_asking: bool) -> Result<ExitCode> {
    let repo = super::checkout_run_in()?;
    let missing: Vec<Requirement> = super::requirements(env, &repo)
        .into_iter()
        .filter(|requirement| requirement.check.status == Status::Fail && requirement.remedy.is_some())
        .collect();

    let asking_for_trust = agents_to_trust_it(&missing);
    if !asking_for_trust.is_empty() && !trust_without_asking && !agrees_to_trust(&repo, &asking_for_trust)? {
        message::summary(Tone::Caution, "Nothing was changed.");
        return Ok(ExitCode::FAILURE);
    }
    for remedy in missing.into_iter().filter_map(|requirement| requirement.remedy) {
        super::apply(remedy)?;
    }

    let now: Vec<Check> = super::requirements(env, &repo)
        .into_iter()
        .map(|requirement| requirement.check)
        .collect();
    for still_open in now.iter().filter(|check| check.status != Status::Ok) {
        message::note(&still_open.detail);
    }
    if !check::is_healthy(&now) {
        message::summary(Tone::Bad, "Not ready yet. `jarvis repo audit` shows what is still missing.");
        return Ok(ExitCode::FAILURE);
    }
    message::summary(Tone::Good, "This repository is ready for a reviewer that fixes what it finds.");
    Ok(ExitCode::SUCCESS)
}

fn agents_to_trust_it(missing: &[Requirement]) -> Vec<String> {
    let grants_trust = |requirement: &&Requirement| requirement.remedy.as_ref().is_some_and(|remedy| remedy.grants_trust);
    missing
        .iter()
        .filter(grants_trust)
        .map(|requirement| requirement.check.area.clone())
        .collect()
}

fn agrees_to_trust(repo: &Repo, agents: &[String]) -> Result<bool> {
    if !Screen::can_open() {
        bail!("`jarvis repo init` asks before it trusts a repository, and this is not an interactive terminal; pass `--yes` to trust it without being asked");
    }
    let question = Picker {
        title: &format!("Trust {} in {}?", repo.root.display(), agents.join(" and ")),
        subtitle: "Every session of theirs here then runs with the repository's own settings, hooks and tools.",
        rows: vec![
            Row {
                label: "Yes, trust it".to_string(),
                note: "and let the reviewer fix what it finds here".to_string(),
                radio: Radio::Absent,
            },
            Row {
                label: "No".to_string(),
                note: "change nothing".to_string(),
                radio: Radio::Absent,
            },
        ],
        cursor: 0,
        help: ANSWER_HELP,
    };
    let answer = question.ask(&mut Screen::open()?)?;
    Ok(matches!(answer, Answer::Chosen(0)))
}
