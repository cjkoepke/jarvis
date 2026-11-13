use crate::git::outgoing::{self, PushedRef};
use crate::git::{self, fix_commit};
use crate::paths::Env;
use crate::rules::{self, ChangedFile, FileFix, Finding, Level, Rule};
use crate::standards::Standards;
use anyhow::Result;
use std::collections::HashSet;
use std::io::Read;
use std::process::ExitCode;

/// @agent Any internal error lets the push through. The review exists to improve
/// what goes out, so it must never be the reason a push cannot happen. Output goes
/// to stderr because git relays a hook's stderr to whoever ran `git push`, which
/// is how an agent receives the result.
pub fn run(env: &Env, git_args: &[String]) -> ExitCode {
    if fix_commit::in_progress() {
        return ExitCode::SUCCESS;
    }
    review(env, git_args).unwrap_or_else(|error| {
        eprintln!("jarvis: review skipped ({error:#}). The push continues.");
        ExitCode::SUCCESS
    })
}

fn review(env: &Env, git_args: &[String]) -> Result<ExitCode> {
    let standards = Standards::load(&env.installed_standards_dir())?;
    let remote_name = git_args.first().map(String::as_str);
    let checked_out_commit = git::run(&["rev-parse", "HEAD"]).ok();

    let mut refs_being_pushed = String::new();
    std::io::stdin().read_to_string(&mut refs_being_pushed)?;

    let mut findings: Vec<Finding> = Vec::new();
    let mut fixes: Vec<FileFix> = Vec::new();
    let mut reviewed_commits = HashSet::new();
    for pushed in refs_being_pushed.lines().filter_map(PushedRef::parse) {
        let already_reviewed = !reviewed_commits.insert(pushed.local_sha.clone());
        if pushed.deletes_the_ref() || already_reviewed {
            continue;
        }
        let files = outgoing::changed_files(&pushed, remote_name)?;
        findings.extend(rules::evaluate(&standards.rules, &files));

        let pushed_commit_is_checked_out = checked_out_commit.as_deref() == Some(pushed.local_sha.as_str());
        if pushed_commit_is_checked_out {
            fixes.extend(fixes_for_files_without_local_edits(&standards.rules, &files)?);
        }
    }

    let blocked = findings.iter().any(|finding| finding.level == Level::Block);
    if blocked {
        fixes.clear();
    }
    print_findings(&findings, &fixes);

    if blocked {
        eprintln!("jarvis: push refused. Fix the `block` items above, commit, and push again.");
        return Ok(ExitCode::FAILURE);
    }
    if !fixes.is_empty() {
        let commit = fix_commit::create(&fixes)?;
        eprintln!(
            "jarvis: fixed {} file(s) in a new commit, {commit} ({}).",
            fixes.len(),
            fix_commit::MESSAGE
        );
        eprintln!(
            "jarvis: push stopped because git cannot add that commit to a push already under way. Run the same push again to send it."
        );
        return Ok(ExitCode::FAILURE);
    }
    if findings.iter().any(|finding| finding.level == Level::Fix) {
        eprintln!("jarvis: the `fix` items above were left alone: the file has uncommitted changes, or the pushed commit is not the one checked out.");
    }
    Ok(ExitCode::SUCCESS)
}

/// @agent A fix is written to the working tree and committed on top of HEAD, so a
/// file with uncommitted edits is skipped rather than overwritten.
fn fixes_for_files_without_local_edits(rules: &[Rule], files: &[ChangedFile]) -> Result<Vec<FileFix>> {
    let mut fixes = Vec::new();
    for fix in files.iter().filter_map(|file| rules::fix(rules, file)) {
        let has_local_edits = !git::run(&["status", "--porcelain", "--", &fix.path])?.is_empty();
        if !has_local_edits {
            fixes.push(fix);
        }
    }
    Ok(fixes)
}

fn print_findings(findings: &[Finding], fixes: &[FileFix]) {
    let fixed_paths: HashSet<&str> = fixes.iter().map(|fix| fix.path.as_str()).collect();
    for finding in findings {
        let label = match finding.level {
            Level::Block => "block",
            Level::Fix if fixed_paths.contains(finding.file.as_str()) => "fixed",
            Level::Fix => "fix",
            Level::Report => "note",
        };
        let location = match finding.line {
            Some(line) => format!("{}:{line}", finding.file),
            None => finding.file.clone(),
        };
        eprintln!("jarvis: {label:<5} {location}: {} [{}]", finding.message, finding.rule_id);
    }
}
