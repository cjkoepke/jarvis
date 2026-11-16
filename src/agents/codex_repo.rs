use super::checks::subject;
use super::{codex, Agent};
use crate::check::requirement::{Remedy, Requirement};
use crate::files;
use crate::git::repo::Repo;
use crate::paths::Env;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use toml_edit::{value, DocumentMut, Item, Table};

const PROJECTS: &str = "projects";
const TRUST_LEVEL: &str = "trust_level";
const TRUSTED: &str = "trusted";
const UNTRUSTED: &str = "untrusted";

#[derive(Debug, PartialEq)]
enum Trust {
    Trusted,
    Refused,
    Undecided,
}

pub fn fixing_requirements<'a>(agent: &dyn Agent, env: &'a Env, repo: &'a Repo) -> Vec<Requirement<'a>> {
    let trust = subject(agent, "trust");
    let file = codex::config_file(env);
    let config = match read(&file) {
        Ok(config) => config,
        Err(error) => return vec![trust.fail(format!("{error:#}; repair that file")).into()],
    };
    let folder = deciding_folder(&config, repo);
    vec![match trust_of(&config, &folder) {
        Trust::Trusted => trust.ok(format!("{folder} is trusted in {}", file.display())).into(),
        Trust::Refused => trust
            .fail(format!(
                "{folder} is marked untrusted in {}, which jarvis leaves alone; set its `trust_level` to \"trusted\" there to let the reviewer fix",
                file.display()
            ))
            .into(),
        Trust::Undecided => trust
            .fail(format!("{folder} is not trusted in {}; run `jarvis repo init`", file.display()))
            .remedied_by(Remedy {
                subject: agent.id(),
                summary: format!("trust for {folder} in {}", file.display()),
                grants_trust: true,
                change: Box::new(move || trust_where_undecided(env, repo)),
            }),
    }]
}

/// Reads the file again, so nothing Codex saved since the requirement was gathered is lost.
fn trust_where_undecided(env: &Env, repo: &Repo) -> Result<()> {
    let file = codex::config_file(env);
    let mut config = read(&file)?;
    let folder = deciding_folder(&config, repo);
    if trust_of(&config, &folder) != Trust::Undecided {
        return Ok(());
    }
    trust(&mut config, &folder).with_context(|| format!("{} cannot hold the trust", file.display()))?;
    files::replace(&file, &config.to_string())
}

pub fn is_trusted(env: &Env, repo: &Repo) -> bool {
    read(&codex::config_file(env)).is_ok_and(|config| trust_of(&config, &deciding_folder(&config, repo)) == Trust::Trusted)
}

fn read(file: &Path) -> Result<DocumentMut> {
    let text = match file.exists() {
        true => fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?,
        false => String::new(),
    };
    text.parse().with_context(|| format!("{} is not valid TOML", file.display()))
}

/// Codex asks about the checkout first and the checkout a worktree was added from second.
/// The first with an entry decides, even when the entry sets no trust level, and Codex
/// records new trust under the last.
fn deciding_folder(config: &DocumentMut, repo: &Repo) -> String {
    let asked_in_order = [Some(repo.root.clone()), repo.main_checkout()];
    let folders: Vec<String> = asked_in_order
        .into_iter()
        .flatten()
        .map(|folder| folder.to_string_lossy().into_owned())
        .collect();
    let first_with_an_entry = folders.iter().find(|folder| entry(config, folder).is_some());
    first_with_an_entry.or(folders.last()).cloned().unwrap_or_default()
}

fn entry<'a>(config: &'a DocumentMut, folder: &str) -> Option<&'a Item> {
    config.get(PROJECTS)?.get(folder)
}

fn trust_of(config: &DocumentMut, folder: &str) -> Trust {
    match entry(config, folder)
        .and_then(|entry| entry.get(TRUST_LEVEL))
        .and_then(Item::as_str)
    {
        Some(TRUSTED) => Trust::Trusted,
        Some(UNTRUSTED) => Trust::Refused,
        _ => Trust::Undecided,
    }
}

fn trust(config: &mut DocumentMut, folder: &str) -> Result<()> {
    let projects = config.entry(PROJECTS).or_insert_with(|| {
        let mut without_a_header_of_its_own = Table::new();
        without_a_header_of_its_own.set_implicit(true);
        Item::Table(without_a_header_of_its_own)
    });
    let projects = projects.as_table_like_mut().context("`projects` is not a table")?;
    if projects.get(folder).is_none() {
        projects.insert(folder, Item::Table(Table::new()));
    }
    let project = projects.get_mut(folder).and_then(Item::as_table_like_mut);
    let project = project.with_context(|| format!("the `projects` entry for {folder} is not a table"))?;
    project.insert(TRUST_LEVEL, value(TRUSTED));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const CONFIG: &str = r#"# my own note
model = "some-model"

[projects."/work/main/worktrees/shadowed"]
approval = "kept"

[projects."/work/main"]
trust_level = "trusted"
"#;

    fn worktree_of_main_at(root: &str) -> Repo {
        Repo {
            root: PathBuf::from(root),
            git_dirs: vec![PathBuf::from("/work/main/.git/worktrees/topic"), PathBuf::from("/work/main/.git")],
            signs_commits: false,
        }
    }

    #[test]
    fn a_worktree_takes_the_trust_of_its_main_checkout_unless_it_has_an_entry_of_its_own() {
        let config: DocumentMut = CONFIG.parse().unwrap();
        let trust_at = |root: &str| trust_of(&config, &deciding_folder(&config, &worktree_of_main_at(root)));

        assert_eq!(trust_at("/work/elsewhere"), Trust::Trusted);
        assert_eq!(trust_at("/work/main/worktrees/shadowed"), Trust::Undecided);
    }

    #[test]
    fn trusting_a_folder_keeps_everything_else_in_the_file() {
        let mut config: DocumentMut = CONFIG.parse().unwrap();
        trust(&mut config, "/work/main/worktrees/shadowed").unwrap();
        trust(&mut config, "/work/new").unwrap();
        let written = config.to_string();

        assert!(written.starts_with(CONFIG.split("approval").next().unwrap()));
        assert!(written.contains("approval = \"kept\"\ntrust_level = \"trusted\"\n"));
        assert!(written.ends_with("[projects.\"/work/new\"]\ntrust_level = \"trusted\"\n"));
    }
}
