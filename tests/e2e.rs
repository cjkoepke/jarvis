use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

const STANDARDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/standards");
const FAKE_REVIEWER: &str = "#!/bin/sh\ncat > \"$(dirname \"$0\")/prompt\"\ncat \"$(dirname \"$0\")/reply\"\n";

/// @agent This harness exists because no third-party crate gives a throwaway home
/// directory together with a git whose global config lives in it. Every git call
/// must go through `git()` so the real user and system config are never read.
struct Sandbox {
    _dir: TempDir,
    home: PathBuf,
}

impl Sandbox {
    fn new() -> Sandbox {
        let dir = TempDir::new().unwrap();
        let home = dir.path().canonicalize().unwrap().join("home");
        fs::create_dir_all(&home).unwrap();
        let sandbox = Sandbox { _dir: dir, home };
        sandbox.install_fake_reviewer();
        sandbox
    }

    fn fake_reviewer_dir(&self) -> PathBuf {
        self.home.join("reviewer")
    }

    fn install_fake_reviewer(&self) {
        let dir = self.fake_reviewer_dir();
        let executable = dir.join("claude");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&executable, FAKE_REVIEWER).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        self.reviewer_replies("VERDICT: PASS");
    }

    fn reviewer_replies(&self, reply: &str) {
        fs::write(self.fake_reviewer_dir().join("reply"), reply).unwrap();
    }

    fn prompt_given_to_reviewer(&self) -> String {
        fs::read_to_string(self.fake_reviewer_dir().join("prompt")).unwrap()
    }

    fn path_with_fake_reviewer_first(&self) -> std::ffi::OsString {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let dirs = std::iter::once(self.fake_reviewer_dir()).chain(std::env::split_paths(&inherited));
        std::env::join_paths(dirs).unwrap()
    }

    fn jarvis(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_jarvis"))
            .arg("--home")
            .arg(&self.home)
            .args(args)
            .output()
            .unwrap()
    }

    fn install(&self) {
        assert!(self.jarvis(&["install", "--standards", STANDARDS]).status.success());
    }

    fn git(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new("git")
            .current_dir(cwd)
            .args(args)
            .env("HOME", &self.home)
            .env("PATH", self.path_with_fake_reviewer_first())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("GIT_CONFIG_GLOBAL")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .unwrap()
    }

    fn git_output(&self, cwd: &Path, args: &[&str]) -> String {
        let output = self.git(cwd, args);
        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn push(&self, repo: &Path) -> Output {
        self.git(repo, &["push", "origin", "main"])
    }

    fn repo_with_one_pushed_commit(&self) -> PathBuf {
        let remote = self.home.join("remote.git");
        let repo = self.home.join("work");
        self.git_output(&self.home, &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
        self.git_output(&self.home, &["init", "--quiet", "-b", "main", repo.to_str().unwrap()]);
        self.git_output(&repo, &["remote", "add", "origin", remote.to_str().unwrap()]);
        self.commit(&repo, "README.md", "hello\n");
        self.git_output(&repo, &["push", "--quiet", "-u", "origin", "main"]);
        repo
    }

    fn commit(&self, repo: &Path, file: &str, contents: &str) {
        fs::write(repo.join(file), contents).unwrap();
        self.git_output(repo, &["add", "--force", file]);
        self.git_output(repo, &["commit", "--quiet", "-m", "change"]);
    }

    fn head(&self, repo: &Path) -> String {
        self.git_output(repo, &["rev-parse", "HEAD"])
    }

    fn remote_head(&self, repo: &Path) -> String {
        self.git_output(repo, &["rev-parse", "origin/main"])
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn registered_hooks_deliver_the_core_to_sessions_and_subagents() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let core = fs::read_to_string(Path::new(STANDARDS).join("core/core.md")).unwrap();

    for hooks_file in [".claude/settings.json", ".codex/hooks.json"] {
        let config = read_json(&sandbox.home.join(hooks_file));
        for event in ["SessionStart", "SubagentStart"] {
            let registered_command = config["hooks"][event][0]["hooks"][0]["command"].as_str().unwrap();
            let reply = Command::new("sh").arg("-c").arg(registered_command).output().unwrap();
            let reply: Value = serde_json::from_slice(&reply.stdout).unwrap();

            assert_eq!(reply["hookSpecificOutput"]["hookEventName"], event);
            assert!(reply["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .unwrap()
                .contains(core.trim()));
        }
    }
}

#[test]
fn install_and_uninstall_leave_existing_configuration_alone() {
    let sandbox = Sandbox::new();
    let settings = sandbox.home.join(".claude/settings.json");
    let own_skill = sandbox.home.join(".claude/skills/commenting/SKILL.md");
    let gitconfig = sandbox.home.join(".gitconfig");
    let settings_before = r#"{"model":"opus","hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo mine"}]}]}}"#;
    fs::create_dir_all(own_skill.parent().unwrap()).unwrap();
    fs::write(&own_skill, "mine\n").unwrap();
    fs::write(&settings, settings_before).unwrap();
    fs::write(&gitconfig, "[user]\n\tname = Someone\n").unwrap();

    sandbox.install();
    sandbox.install();
    let installed = read_json(&settings);
    assert_eq!(installed["hooks"]["SessionStart"].as_array().unwrap().len(), 2);
    assert_eq!(installed["hooks"]["SessionStart"][0]["hooks"][0]["command"], "echo mine");
    assert_eq!(installed["hooks"]["SubagentStart"].as_array().unwrap().len(), 1);
    assert_eq!(fs::read_to_string(&own_skill).unwrap(), "mine\n");

    assert!(sandbox.jarvis(&["install", "--agents", "claude"]).status.success());
    assert!(!sandbox.home.join(".codex/hooks.json").exists());
    assert!(!sandbox.home.join(".agents/skills/commenting").exists());

    assert!(sandbox.jarvis(&["uninstall"]).status.success());
    assert_eq!(read_json(&settings), serde_json::from_str::<Value>(settings_before).unwrap());
    assert_eq!(fs::read_to_string(&own_skill).unwrap(), "mine\n");
    assert_eq!(
        sandbox.git_output(&sandbox.home, &["config", "--global", "--list"]),
        "user.name=Someone"
    );
    assert!(!sandbox.home.join(".codex/hooks.json").exists());
    assert!(!sandbox.home.join(".agents/skills/commenting").exists());
    assert!(!sandbox.home.join(".jarvis").exists());
}

#[test]
fn reviewer_is_given_the_standards_and_only_the_commits_the_remote_lacks() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let core = fs::read_to_string(Path::new(STANDARDS).join("core/core.md")).unwrap();

    sandbox.commit(&repo, "first.rs", "fn first_unpushed() {}\n");
    sandbox.commit(&repo, "second.rs", "fn second_unpushed() {}\n");
    assert!(sandbox.push(&repo).status.success());

    let prompt = sandbox.prompt_given_to_reviewer();
    assert!(prompt.contains(core.trim()));
    assert!(prompt.contains("fn first_unpushed() {}") && prompt.contains("fn second_unpushed() {}"));
    assert!(!prompt.contains("+hello"));
}

#[test]
fn reviewer_decides_the_push_and_a_broken_reviewer_never_stops_one() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let last_pushed = sandbox.head(&repo);

    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    sandbox.reviewer_replies("lib.rs:1 breaks a standard\nVERDICT: FAIL");
    assert!(!sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), last_pushed);

    sandbox.reviewer_replies("VERDICT: PASS");
    assert!(sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), sandbox.head(&repo));

    sandbox.commit(&repo, "more.rs", "fn more() {}\n");
    sandbox.reviewer_replies("I could not finish the review.");
    assert!(sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), sandbox.head(&repo));
}
