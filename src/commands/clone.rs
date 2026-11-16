use crate::paths::Env;
use crate::record::InstallRecord;
use crate::standards::custom;
use crate::standards::source::Source;
use crate::ui::prompt::{Answer, Checklist, Item, Screen};
use crate::ui::{message, Tone};
use anyhow::{bail, Result};
use std::process::ExitCode;

pub fn run(env: &Env, named: &[String]) -> Result<ExitCode> {
    let source = InstallRecord::standards_in_effect(InstallRecord::load(env).as_ref());
    let given = tempfile::tempdir()?;
    source.copy_into(given.path())?;
    let offered = custom::pieces(given.path())?;
    let custom_dir = env.custom_standards_dir();
    let yours = custom::pieces(&custom_dir)?;
    let is_in_the_way = |piece: &String| custom_dir.join(piece).symlink_metadata().is_ok();

    let wanted = match named {
        [] => ticked(&offered, &source, |piece| yours.contains(piece))?,
        named => offered_among(named, &offered, &source)?,
    };
    let mut cloned_any = false;
    for piece in &wanted {
        let destination = custom_dir.join(piece);
        if yours.contains(piece) {
            message::note(&format!(
                "{piece} is already yours and was left as it is; remove {} to take it again",
                destination.display()
            ));
            continue;
        }
        if is_in_the_way(piece) {
            message::note(&format!(
                "{} is in the way and is not a usable {piece}; repair or remove it",
                destination.display()
            ));
            continue;
        }
        custom::copy_piece(&given.path().join(piece), &destination)?;
        message::done("clone", &format!("{piece} to {}", destination.display()));
        cloned_any = true;
    }
    if cloned_any {
        message::summary(Tone::Good, "Change them there, then run `jarvis install` to deliver them.");
    }
    Ok(ExitCode::SUCCESS)
}

fn offered_among(named: &[String], offered: &[String], source: &Source) -> Result<Vec<String>> {
    if let Some(unknown) = named.iter().find(|piece| !offered.contains(piece)) {
        bail!("{source} have no `{unknown}`; they have {}", offered.join(", "));
    }
    Ok(named.to_vec())
}

fn ticked(offered: &[String], source: &Source, is_already_yours: impl Fn(&String) -> bool) -> Result<Vec<String>> {
    if !Screen::can_open() {
        bail!(
            "name what to clone, since this is not an interactive terminal: {}",
            offered.join(", ")
        );
    }
    let items = offered.iter().map(|piece| Item {
        label: piece.clone(),
        note: if is_already_yours(piece) { "already yours" } else { "" }.to_string(),
        checked: false,
    });
    let question = Checklist {
        title: "Clone which of these?",
        subtitle: &format!("From {source}. Your copy then replaces the original, which stops changing with jarvis."),
        items: items.collect(),
    };
    let answer = question.ask(&mut Screen::open()?)?;
    Ok(match answer {
        Answer::Chosen(ticked) => ticked.into_iter().map(|item| offered[item].clone()).collect(),
        Answer::Back | Answer::Quit => Vec::new(),
    })
}
