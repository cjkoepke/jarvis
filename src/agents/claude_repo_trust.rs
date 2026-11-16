use super::json_config;
use crate::git::repo::Repo;
use crate::paths::Env;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const PROJECTS: &str = "projects";
const TRUSTED: &str = "hasTrustDialogAccepted";

const LOCK_SUFFIX: &str = ".lock";
const LOCK_WAIT: Duration = Duration::from_secs(5);
const LOCK_POLL: Duration = Duration::from_millis(100);
const LOCK_IS_ABANDONED_AFTER: Duration = Duration::from_secs(10);

pub fn file(env: &Env) -> PathBuf {
    env.home.join(".claude.json")
}

/// The folder Claude Code looks a checkout's trust up under. A parent folder's trust does
/// not count, and a linked worktree shares the trust of the repository it was added from.
pub fn folder(repo: &Repo) -> PathBuf {
    let repository_of_a_worktree = repo.main_checkout().or(repo.shared_git_dir().map(Path::to_path_buf));
    repository_of_a_worktree.unwrap_or_else(|| repo.root.clone())
}

pub fn is_trusted(env: &Env, repo: &Repo) -> Result<bool> {
    let config = json_config::read(&file(env))?;
    let project = config.get(PROJECTS).and_then(|projects| projects.get(key(repo)));
    Ok(project.and_then(|project| project.get(TRUSTED)) == Some(&Value::Bool(true)))
}

pub fn trust(env: &Env, repo: &Repo) -> Result<()> {
    if is_trusted(env, repo)? {
        return Ok(());
    }
    let file = file(env);
    let file = file.canonicalize().unwrap_or(file);
    let _lock = Lock::take(&file)?;

    let mut config = json_config::read(&file)?;
    let projects = config.entry(PROJECTS).or_insert_with(|| json!({}));
    let projects = projects
        .as_object_mut()
        .with_context(|| format!("`{PROJECTS}` in {} is not an object", file.display()))?;
    let project = projects.entry(key(repo)).or_insert_with(|| json!({}));
    let project = project
        .as_object_mut()
        .with_context(|| format!("the entry for {} in {} is not an object", key(repo), file.display()))?;
    project.insert(TRUSTED.to_string(), Value::Bool(true));
    json_config::write(&file, &config)
}

fn key(repo: &Repo) -> String {
    folder(repo).to_string_lossy().into_owned()
}

/// The lock Claude Code itself holds while it saves this file: a folder beside it.
struct Lock {
    dir: PathBuf,
}

impl Lock {
    fn take(file: &Path) -> Result<Lock> {
        let mut dir = file.as_os_str().to_owned();
        dir.push(LOCK_SUFFIX);
        let dir = PathBuf::from(dir);

        let deadline = Instant::now() + LOCK_WAIT;
        loop {
            match fs::create_dir(&dir) {
                Ok(()) => return Ok(Lock { dir }),
                Err(error) if error.kind() == ErrorKind::AlreadyExists && Instant::now() < deadline => {
                    if is_abandoned(&dir) {
                        fs::remove_dir(&dir).ok();
                    } else {
                        thread::sleep(LOCK_POLL);
                    }
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "Claude Code is saving {}, going by {}; try again in a moment",
                            file.display(),
                            dir.display()
                        )
                    });
                }
            }
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        fs::remove_dir(&self.dir).ok();
    }
}

fn is_abandoned(lock_dir: &Path) -> bool {
    let last_refreshed = fs::metadata(lock_dir).and_then(|lock| lock.modified()).ok();
    let age = last_refreshed.and_then(|refreshed| SystemTime::now().duration_since(refreshed).ok());
    age.is_some_and(|age| age > LOCK_IS_ABANDONED_AFTER)
}
