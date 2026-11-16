use crate::check::requirement::Remedy;
use crate::check::{self, Check, Status};
use crate::cli::Format;
use crate::git::repo::Repo;
use crate::paths::Env;
use crate::ui::prompt::{Answer, Checklist, Item, Screen};
use anyhow::Result;
use std::process::ExitCode;

pub fn run(env: &Env, format: Format) -> Result<ExitCode> {
    let repo = super::checkout_run_in()?;
    let (checks, remedies) = checks_and_remedies(env, &repo);
    let outcome = check::report::print(&checks, format)?;

    let can_offer_fixes = matches!(format, Format::Table) && Screen::can_open();
    if !can_offer_fixes || remedies.iter().all(Option::is_none) {
        return Ok(outcome);
    }
    let chosen = fixes_chosen(&checks, &remedies)?;
    if chosen.is_empty() {
        return Ok(outcome);
    }
    println!();
    for (index, remedy) in remedies.into_iter().enumerate() {
        if let Some(remedy) = remedy.filter(|_| chosen.contains(&index)) {
            super::apply(remedy)?;
        }
    }
    let (checks, _) = checks_and_remedies(env, &repo);
    check::report::print(&checks, format)
}

fn checks_and_remedies<'a>(env: &'a Env, repo: &'a Repo) -> (Vec<Check>, Vec<Option<Remedy<'a>>>) {
    let requirements = super::requirements(env, repo).into_iter();
    requirements.map(|requirement| (requirement.check, requirement.remedy)).unzip()
}

/// The indexes of the checks whose remedy the person wants applied.
fn fixes_chosen(checks: &[Check], remedies: &[Option<Remedy>]) -> Result<Vec<usize>> {
    let fixable: Vec<(usize, &Check, &Remedy)> = checks
        .iter()
        .zip(remedies)
        .enumerate()
        .filter_map(|(index, (check, remedy))| Some((index, check, remedy.as_ref()?)))
        .collect();
    let items = fixable.iter().map(|(_, check, remedy)| Item {
        label: format!("{} {}", check.area, check.name),
        note: remedy.summary.clone(),
        checked: check.status == Status::Fail,
    });
    let question = Checklist {
        title: "Fix these now?",
        subtitle: "Each checked line is put in place when you confirm.",
        items: items.collect(),
    };
    let answer = question.ask(&mut Screen::open()?)?;
    Ok(match answer {
        Answer::Chosen(checked) => checked.into_iter().map(|item| fixable[item].0).collect(),
        Answer::Back | Answer::Quit => Vec::new(),
    })
}
