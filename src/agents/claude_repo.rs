use super::checks::subject;
use super::{claude_repo_trust, json_config, Agent};
use crate::check::requirement::{Remedy, Requirement};
use crate::git::repo::Repo;
use crate::paths::Env;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const FIXING_RULES: [&str; 6] = [
    "Edit(/**)",
    "Bash(sed *)",
    "Bash(git status *)",
    "Bash(git add *)",
    "Bash(git commit *)",
    "Bash(git restore *)",
];

const PERMISSIONS: &str = "permissions";
const ALLOWED: &str = "allow";

pub fn fixing_requirements<'a>(agent: &dyn Agent, env: &'a Env, repo: &'a Repo) -> Vec<Requirement<'a>> {
    vec![trust_requirement(agent, env, repo), permissions_requirement(agent, env, repo)]
}

fn trust_requirement<'a>(agent: &dyn Agent, env: &'a Env, repo: &'a Repo) -> Requirement<'a> {
    let trust = subject(agent, "trust");
    let folder = claude_repo_trust::folder(repo);
    let file = claude_repo_trust::file(env);
    match claude_repo_trust::is_trusted(env, repo) {
        Ok(true) => trust.ok(format!("{} is trusted in {}", folder.display(), file.display())).into(),
        Ok(false) => trust
            .fail(format!(
                "{} is not trusted in {}, so its permission rules are ignored; run `jarvis repo init`",
                folder.display(),
                file.display()
            ))
            .remedied_by(Remedy {
                subject: agent.id(),
                summary: format!("trust for {} in {}", folder.display(), file.display()),
                grants_trust: true,
                change: Box::new(|| claude_repo_trust::trust(env, repo)),
            }),
        Err(error) => trust.fail(format!("{error:#}; repair that file")).into(),
    }
}

fn permissions_requirement<'a>(agent: &dyn Agent, env: &'a Env, repo: &'a Repo) -> Requirement<'a> {
    let permissions = subject(agent, "permissions");
    let mut allowed = Vec::new();
    for settings_file in settings_read_for(env, repo) {
        match allowed_rules(&settings_file) {
            Ok(rules) => allowed.extend(rules),
            Err(error) => return permissions.fail(format!("{error:#}; repair that file")).into(),
        }
    }
    let not_allowed = fixing_rules_missing_from(&allowed);
    if not_allowed.is_empty() {
        return permissions.ok("the reviewer may edit and commit").into();
    }
    let settings_file = repo_settings(repo);
    permissions
        .fail(format!(
            "{} not allowed in {}; run `jarvis repo init`",
            not_allowed.join(", "),
            settings_file.display()
        ))
        .remedied_by(Remedy {
            subject: agent.id(),
            summary: format!("rules to edit and commit in {}", settings_file.display()),
            grants_trust: false,
            change: Box::new(move || add_fixing_rules(&settings_file)),
        })
}

fn repo_settings(repo: &Repo) -> PathBuf {
    repo.root.join(".claude/settings.json")
}

fn settings_read_for(env: &Env, repo: &Repo) -> [PathBuf; 3] {
    [
        repo_settings(repo),
        repo.root.join(".claude/settings.local.json"),
        env.home.join(".claude/settings.json"),
    ]
}

fn allowed_rules(settings_file: &Path) -> Result<Vec<Value>> {
    let settings = json_config::read(settings_file)?;
    let allowed = settings.get(PERMISSIONS).and_then(|permissions| permissions.get(ALLOWED));
    Ok(allowed.and_then(Value::as_array).cloned().unwrap_or_default())
}

fn fixing_rules_missing_from(allowed: &[Value]) -> Vec<&'static str> {
    let is_allowed = |rule: &&str| allowed.iter().any(|allowed| allowed == rule);
    FIXING_RULES.into_iter().filter(|rule| !is_allowed(rule)).collect()
}

fn add_fixing_rules(settings_file: &Path) -> Result<()> {
    let mut settings = json_config::read(settings_file)?;
    let permissions = settings.entry(PERMISSIONS).or_insert_with(|| json!({}));
    let permissions = permissions
        .as_object_mut()
        .with_context(|| format!("`{PERMISSIONS}` in {} is not an object", settings_file.display()))?;
    let allowed = permissions.entry(ALLOWED).or_insert_with(|| json!([]));
    let allowed = allowed
        .as_array_mut()
        .with_context(|| format!("`{PERMISSIONS}.{ALLOWED}` in {} is not a list", settings_file.display()))?;

    let not_allowed = fixing_rules_missing_from(allowed);
    allowed.extend(not_allowed.into_iter().map(Value::from));
    json_config::write(settings_file, &settings)
}
