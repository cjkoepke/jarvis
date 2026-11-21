use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const STANDARDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/standards");
const FAKE_BREW: &str = r#"#!/bin/sh
PATH=/usr/bin:/bin
programs="${0%/*}"
case "$1" in
    install) printf '#!/bin/sh\n' > "$programs/ast-grep" && chmod 755 "$programs/ast-grep" ;;
    uninstall) rm "$programs/ast-grep" ;;
    *) exit 1 ;;
esac
echo "$1" >> "$programs/brew-runs"
"#;
const FAKE_REVIEWER: &str = r#"#!/bin/sh
dir="$(dirname "$0")"
cat > "$dir/prompt"
git log --format=%H $JARVIS_PUSH_COMMITS --not --remotes="$JARVIS_PUSH_REMOTE" > "$dir/commits"
if [ -f "$dir/fix" ] && [ "$JARVIS_REVIEW_FIX" = true ]; then
    mv "$dir/fix" fixed.rs
    git add fixed.rs
    git commit --quiet -m "fix: follow the standards"
fi
cat "$dir/reply"
"#;

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
        sandbox.stand_in_for_machine_programs();
        sandbox
    }

    fn fake_programs_dir(&self) -> PathBuf {
        self.home.join("programs")
    }

    fn install_fake_reviewer(&self) {
        fs::create_dir_all(self.fake_programs_dir()).unwrap();
        self.write_program("claude", FAKE_REVIEWER);
        self.reviewer_replies("VERDICT: PASS");
    }

    fn stand_in_for_machine_programs(&self) {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let real_git = std::env::split_paths(&inherited)
            .map(|dir| dir.join("git"))
            .find(|git| git.is_file())
            .expect("the tests need git on PATH");
        symlink(real_git, self.fake_programs_dir().join("git")).unwrap();
        self.write_program("ast-grep", "#!/bin/sh\n");
    }

    /// Returns the file the fake brew lists its installs and uninstalls in, one verb per line.
    fn fake_brew(&self) -> PathBuf {
        self.write_program("brew", FAKE_BREW);
        self.fake_programs_dir().join("brew-runs")
    }

    fn write_program(&self, name: &str, script: &str) {
        let executable = self.fake_programs_dir().join(name);
        fs::write(&executable, script).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn reviewer_replies(&self, reply: &str) {
        fs::write(self.fake_programs_dir().join("reply"), reply).unwrap();
    }

    fn reviewer_commits_a_fix(&self) {
        fs::write(self.fake_programs_dir().join("fix"), "fn fixed() {}\n").unwrap();
    }

    fn prompt_given_to_reviewer(&self) -> String {
        fs::read_to_string(self.fake_programs_dir().join("prompt")).unwrap()
    }

    fn commits_the_reviewer_found(&self) -> String {
        fs::read_to_string(self.fake_programs_dir().join("commits")).unwrap()
    }

    fn path_with_fake_programs_first(&self) -> std::ffi::OsString {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let dirs = std::iter::once(self.fake_programs_dir()).chain(std::env::split_paths(&inherited));
        std::env::join_paths(dirs).unwrap()
    }

    fn with_only_the_fake_programs<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command.env("PATH", self.fake_programs_dir())
    }

    fn jarvis(&self, args: &[&str]) -> Output {
        let mut jarvis = Command::new(env!("CARGO_BIN_EXE_jarvis"));
        jarvis.arg("--home").arg(&self.home).args(args);
        self.with_only_the_fake_programs(&mut jarvis).output().unwrap()
    }

    fn jarvis_in(&self, repo: &Path, args: &[&str]) -> Output {
        let mut jarvis = Command::new(env!("CARGO_BIN_EXE_jarvis"));
        jarvis.current_dir(repo).arg("--home").arg(&self.home).args(args);
        self.with_only_the_fake_programs(&mut jarvis);
        self.with_only_the_sandbox_git_config(&mut jarvis).output().unwrap()
    }

    fn with_only_the_sandbox_git_config<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command
            .env("HOME", &self.home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("GIT_CONFIG_GLOBAL")
    }

    fn install(&self) {
        assert!(self.jarvis(&["install", "--standards", STANDARDS]).status.success());
    }

    /// Runs a hook command the way the agent does, with `input` on stdin.
    fn run_hook(&self, command: &str, input: &Value) -> Output {
        let mut hook = Command::new("/bin/sh");
        hook.arg("-c").arg(command).stdin(Stdio::piped()).stdout(Stdio::piped());
        let mut running = self.with_only_the_fake_programs(&mut hook).spawn().unwrap();
        running.stdin.take().unwrap().write_all(input.to_string().as_bytes()).unwrap();
        running.wait_with_output().unwrap()
    }

    fn git(&self, cwd: &Path, args: &[&str]) -> Output {
        let mut git = Command::new("git");
        git.current_dir(cwd).args(args);
        self.with_only_the_sandbox_git_config(&mut git)
            .env("PATH", self.path_with_fake_programs_first())
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

    fn kept_review_of(&self, commit: &str) -> PathBuf {
        let reviews = fs::read_dir(self.home.join(".jarvis/reviews")).unwrap();
        let mut reviews = reviews.map(|review| review.unwrap().path());
        reviews
            .find(|review| read_json(&review.join("review.json"))["commits"][0] == commit)
            .unwrap()
    }

    fn fixing_recorded_for(&self, commit: &str) -> Value {
        read_json(&self.kept_review_of(commit).join("review.json"))["fixing"].clone()
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn append_line(file: &Path, entry: Value) {
    let mut file = fs::OpenOptions::new().create(true).append(true).open(file).unwrap();
    writeln!(file, "{entry}").unwrap();
}

fn typed_prompt(text: &str) -> Value {
    json!({ "type": "user", "origin": { "kind": "human" }, "message": { "role": "user", "content": text } })
}

fn answered_question(question: &str, labels: &[&str], answer: &str) -> Value {
    let options: Vec<Value> = labels.iter().map(|label| json!({ "label": label })).collect();
    json!({
        "type": "user",
        "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "answered" }] },
        "toolUseResult": { "questions": [{ "question": question, "options": options }], "answers": { (question): answer } }
    })
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
            let mut hook = Command::new("/bin/sh");
            hook.arg("-c").arg(registered_command);
            let reply = sandbox.with_only_the_fake_programs(&mut hook).output().unwrap();
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
fn starting_agents_waits_until_the_owner_picks_a_tier_the_standards_offer() {
    let sandbox = Sandbox::new();
    let standards = sandbox.home.join("standards");
    fs::create_dir_all(standards.join("core")).unwrap();
    fs::create_dir_all(standards.join("skills/orchestration")).unwrap();
    fs::write(standards.join("core/core.md"), "Be brief.\n").unwrap();
    fs::write(
        standards.join("skills/orchestration/SKILL.md"),
        "- Offer these options:\n  - Thrifty: one agent.\n  - Lavish: ten agents.\n",
    )
    .unwrap();
    assert!(sandbox
        .jarvis(&["install", "--standards", standards.to_str().unwrap()])
        .status
        .success());

    let gate = &read_json(&sandbox.home.join(".claude/settings.json"))["hooks"]["PreToolUse"][0];
    let gated_tools: Vec<&str> = gate["matcher"].as_str().unwrap().split('|').collect();
    assert!(gated_tools.contains(&"Agent") && gated_tools.contains(&"Workflow"));
    let transcript = sandbox.home.join("session.jsonl");
    let launch = |agent_id: Option<&str>| {
        let input = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Agent",
            "transcript_path": transcript,
            "agent_id": agent_id,
        });
        let reply = sandbox.run_hook(gate["hooks"][0]["command"].as_str().unwrap(), &input);
        assert!(reply.status.success());
        if reply.stdout.is_empty() {
            return None;
        }
        let reply: Value = serde_json::from_slice(&reply.stdout).unwrap();
        assert_eq!(reply["hookSpecificOutput"]["permissionDecision"], "deny");
        Some(
            reply["hookSpecificOutput"]["permissionDecisionReason"]
                .as_str()
                .unwrap()
                .to_string(),
        )
    };

    append_line(&transcript, typed_prompt("research this with a few agents"));
    let reason = launch(None).unwrap();
    assert!(reason.contains("Thrifty: one agent.") && reason.contains("Lavish: ten agents."));
    assert_eq!(launch(Some("subagent")), None);

    append_line(&transcript, answered_question("Which hotel?", &["Thrifty", "Downtown"], "Thrifty"));
    assert!(launch(None).is_some());
    let tiers = ["Thrifty (Recommended)", "Lavish"];
    append_line(&transcript, answered_question("Which tier?", &tiers, "Please skip Lavish"));
    assert!(launch(None).is_some());
    append_line(&transcript, answered_question("Which tier?", &tiers, tiers[0]));
    assert_eq!(launch(None), None);
    append_line(
        &transcript,
        json!({
            "type": "user", "origin": { "kind": "task-notification" }, "turnOrigin": "human",
            "message": { "role": "user", "content": "done" }
        }),
    );
    assert_eq!(launch(None), None);

    append_line(&transcript, typed_prompt("now something else"));
    assert!(launch(None).is_some());
    append_line(&transcript, typed_prompt("lavish, go"));
    assert_eq!(launch(None), None);
}

#[test]
fn install_and_uninstall_leave_existing_configuration_alone() {
    let sandbox = Sandbox::new();
    let settings = sandbox.home.join(".claude/settings.json");
    let own_skill = sandbox.home.join(".claude/skills/commenting/SKILL.md");
    let gitconfig = sandbox.home.join(".gitconfig");
    let brew_runs = sandbox.fake_brew();
    let settings_before = r#"{"model":"opus","hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo mine"}]}],"PreToolUse":[{"matcher":"Agent","hooks":[{"type":"command","command":"echo mine"}]}]}}"#;
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
    assert_eq!(installed["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
    assert_eq!(installed["hooks"]["PreToolUse"][0]["hooks"][0]["command"], "echo mine");
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
    assert!(!brew_runs.exists());
}

#[test]
fn an_ast_grep_an_install_put_in_place_outlasts_a_reinstall_and_leaves_with_uninstall() {
    let sandbox = Sandbox::new();
    let ast_grep = sandbox.fake_programs_dir().join("ast-grep");
    fs::remove_file(&ast_grep).unwrap();
    let brew_runs = sandbox.fake_brew();

    sandbox.install();
    sandbox.install();
    assert!(ast_grep.is_file());
    assert_eq!(fs::read_to_string(&brew_runs).unwrap(), "install\n");

    assert!(sandbox.jarvis(&["uninstall"]).status.success());
    assert_eq!(fs::read_to_string(&brew_runs).unwrap(), "install\nuninstall\n");
    assert!(!ast_grep.exists());
}

#[test]
fn reviewer_gets_the_core_the_review_skill_and_only_the_commits_the_remote_lacks() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let core = fs::read_to_string(Path::new(STANDARDS).join("core/core.md")).unwrap();
    let review_skill = fs::read_to_string(Path::new(STANDARDS).join("skills/review/SKILL.md")).unwrap();

    sandbox.commit(&repo, "first.rs", "fn first_unpushed() {}\n");
    let first = sandbox.head(&repo);
    sandbox.commit(&repo, "second.rs", "fn second_unpushed() {}\n");
    let second = sandbox.head(&repo);
    assert!(sandbox.push(&repo).status.success());

    let prompt = sandbox.prompt_given_to_reviewer();
    assert!(prompt.contains(core.trim()) && prompt.contains(review_skill.trim()));
    assert_eq!(sandbox.commits_the_reviewer_found(), format!("{second}\n{first}\n"));
}

#[test]
fn settings_in_the_jarvis_folder_choose_the_reviewer_and_outlast_an_install() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let settings = sandbox.home.join(".jarvis/settings.toml");
    let codex = sandbox.fake_programs_dir().join("codex");
    let codex_ran = sandbox.fake_programs_dir().join("codex-ran");
    fs::write(&codex, "#!/bin/sh\ntouch \"$(dirname \"$0\")/codex-ran\"\necho 'VERDICT: PASS'\n").unwrap();
    fs::set_permissions(&codex, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(&settings, "[review]\nagent = \"codex\"\n").unwrap();

    sandbox.install();
    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    assert!(sandbox.push(&repo).status.success());

    assert!(codex_ran.exists());
    assert_eq!(fs::read_to_string(&settings).unwrap(), "[review]\nagent = \"codex\"\n");
}

#[test]
fn reviewer_decides_the_push_and_a_broken_reviewer_never_stops_one() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();

    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    sandbox.reviewer_replies("VERDICT: PASS");
    assert!(sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), sandbox.head(&repo));

    sandbox.commit(&repo, "more.rs", "fn more() {}\n");
    sandbox.reviewer_replies("I could not finish the review.");
    assert!(sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), sandbox.head(&repo));
    let last_pushed = sandbox.head(&repo);

    sandbox.commit(&repo, "bad.rs", "fn bad() {}\n");
    sandbox.reviewer_replies("bad.rs:1 breaks a standard\nVERDICT: FAIL");
    assert!(!sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), last_pushed);
}

#[test]
fn a_refused_push_goes_out_unreviewed_on_the_next_attempt_and_the_review_returns_after_it() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let prompt = sandbox.fake_programs_dir().join("prompt");
    sandbox.reviewer_replies("lib.rs:1 breaks a standard\nVERDICT: FAIL");

    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    assert!(!sandbox.push(&repo).status.success());

    fs::remove_file(&prompt).unwrap();
    sandbox.commit(&repo, "lib.rs", "fn fixed() {}\n");
    assert!(sandbox.push(&repo).status.success());
    assert!(!prompt.exists());
    assert_eq!(sandbox.remote_head(&repo), sandbox.head(&repo));
    let last_pushed = sandbox.head(&repo);

    sandbox.commit(&repo, "more.rs", "fn more() {}\n");
    assert!(!sandbox.push(&repo).status.success());
    assert!(prompt.exists());
    assert_eq!(sandbox.remote_head(&repo), last_pushed);
}

#[test]
fn a_refusal_is_spent_by_any_push_of_that_ref_that_is_let_through() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let prompt = sandbox.fake_programs_dir().join("prompt");
    sandbox.reviewer_replies("VERDICT: FAIL");
    let push_topic = |refspec: &str| sandbox.git(&repo, &["push", "origin", refspec]).status.success();

    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    assert!(!push_topic("HEAD:refs/heads/topic"));
    assert!(push_topic("HEAD~1:refs/heads/topic"));
    assert!(push_topic(":refs/heads/topic"));

    fs::remove_file(&prompt).unwrap();
    assert!(!push_topic("HEAD:refs/heads/topic"));
    assert!(prompt.exists());
}

#[test]
fn a_reviewer_allowed_to_fix_commits_on_top_and_the_next_push_sends_its_fix() {
    let sandbox = Sandbox::new();
    sandbox.install();
    fs::write(sandbox.home.join(".jarvis/settings.toml"), "[review]\nfix = true\n").unwrap();
    let repo = sandbox.repo_with_one_pushed_commit();
    let last_pushed = sandbox.head(&repo);
    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    let unfixed = sandbox.head(&repo);

    sandbox.reviewer_commits_a_fix();
    sandbox.reviewer_replies("lib.rs:1 fixed\nVERDICT: FIXED");
    assert!(!sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), last_pushed);
    assert_eq!(sandbox.git_output(&repo, &["rev-parse", "HEAD~1"]), unfixed);

    assert!(sandbox.push(&repo).status.success());
    assert_eq!(sandbox.remote_head(&repo), sandbox.head(&repo));
}

#[test]
fn a_reviewer_does_not_fix_a_push_of_refs_its_commits_would_not_follow() {
    let sandbox = Sandbox::new();
    sandbox.install();
    fs::write(sandbox.home.join(".jarvis/settings.toml"), "[review]\nfix = true\n").unwrap();
    let repo = sandbox.repo_with_one_pushed_commit();
    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    let unfixed = sandbox.head(&repo);
    sandbox.git_output(&repo, &["branch", "topic"]);
    sandbox.reviewer_commits_a_fix();
    sandbox.reviewer_replies("lib.rs:1 breaks a standard\nVERDICT: FAIL");

    for refs in [&["topic"][..], &["main", "main:refs/heads/other"]] {
        assert!(!sandbox.git(&repo, &[&["push", "origin"], refs].concat()).status.success());
        assert_eq!(sandbox.head(&repo), unfixed);
    }
}

#[test]
fn every_review_is_kept_with_what_ruled_out_a_fix() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let fixing_when_pushing = |file: &str, refs: &[&str]| {
        sandbox.commit(&repo, file, "fn main() {}\n");
        sandbox.git_output(&repo, &["branch", "--force", "topic"]);
        assert!(sandbox.git(&repo, &[&["push", "origin"], refs].concat()).status.success());
        sandbox.fixing_recorded_for(&sandbox.head(&repo))
    };

    assert_eq!(fixing_when_pushing("off.rs", &["main"]), "switched-off");
    let kept = sandbox.kept_review_of(&sandbox.head(&repo));
    assert_eq!(
        fs::read_to_string(kept.join("prompt.md")).unwrap(),
        sandbox.prompt_given_to_reviewer()
    );
    assert_eq!(fs::read_to_string(kept.join("reply.md")).unwrap(), "VERDICT: PASS");
    assert_eq!(read_json(&kept.join("review.json"))["remote"], "origin");

    fs::write(sandbox.home.join(".jarvis/settings.toml"), "[review]\nfix = true\n").unwrap();
    assert_eq!(fixing_when_pushing("on.rs", &["main"]), "allowed");
    assert_eq!(fixing_when_pushing("two.rs", &["main", "main:refs/heads/other"]), "several-refs");
    assert_eq!(fixing_when_pushing("topic.rs", &["topic"]), "ref-not-checked-out");
    fs::write(repo.join("README.md"), "an uncommitted edit\n").unwrap();
    assert_eq!(fixing_when_pushing("dirty.rs", &["main"]), "uncommitted-edits");
}

#[test]
fn review_metadata_omits_a_remote_given_as_a_url() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    sandbox.commit(&repo, "direct.rs", "fn main() {}\n");
    let remote = sandbox.home.join("remote.git");

    assert!(sandbox.git(&repo, &["push", remote.to_str().unwrap(), "main"]).status.success());

    let kept = sandbox.kept_review_of(&sandbox.head(&repo));
    assert_eq!(read_json(&kept.join("review.json"))["remote"], Value::Null);
}

#[test]
fn a_fixing_reviewer_finds_the_gpg_agent_started_when_the_repository_signs_commits() {
    let sandbox = Sandbox::new();
    sandbox.install();
    fs::write(sandbox.home.join(".jarvis/settings.toml"), "[review]\nfix = true\n").unwrap();
    let fake_gpgconf = sandbox.fake_programs_dir().join("gpgconf");
    let gpgconf_args = sandbox.fake_programs_dir().join("gpgconf-args");
    fs::write(&fake_gpgconf, "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/gpgconf-args\"\n").unwrap();
    fs::set_permissions(&fake_gpgconf, fs::Permissions::from_mode(0o755)).unwrap();
    let repo = sandbox.repo_with_one_pushed_commit();
    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    sandbox.git_output(&repo, &["config", "commit.gpgsign", "true"]);

    assert!(sandbox.push(&repo).status.success());
    assert_eq!(fs::read_to_string(gpgconf_args).unwrap().trim(), "--launch gpg-agent");
}

#[test]
fn install_falls_back_to_the_packaged_standards_when_the_last_folder_is_gone() {
    let sandbox = Sandbox::new();
    let record_file = sandbox.home.join(".jarvis/install.json");
    fs::create_dir_all(record_file.parent().unwrap()).unwrap();
    fs::write(
        &record_file,
        r#"{ "standards_source": "/a/folder/that/is/gone", "agent_ids": ["claude"] }"#,
    )
    .unwrap();

    assert!(sandbox.jarvis(&["install"]).status.success());
    assert!(read_json(&record_file)["standards_source"].is_null());
    assert_eq!(read_json(&record_file)["agent_ids"], serde_json::json!(["claude"]));
    assert!(sandbox.home.join(".jarvis/standards/skills/review/SKILL.md").is_file());
}

#[test]
fn install_refuses_sources_inside_a_symlinked_jarvis_directory() {
    let sandbox = Sandbox::new();
    let storage = sandbox.home.join("storage");
    fs::create_dir_all(&storage).unwrap();
    symlink(&storage, sandbox.home.join(".jarvis")).unwrap();
    assert!(sandbox.jarvis(&["install", "--packaged"]).status.success());
    let installed = storage.join("standards");
    let record = fs::read(storage.join("install.json")).unwrap();

    assert!(!sandbox
        .jarvis(&["install", "--standards", installed.to_str().unwrap()])
        .status
        .success());
    assert_eq!(fs::read(storage.join("install.json")).unwrap(), record);
}

#[test]
fn your_own_standards_replace_and_extend_the_packaged_ones_and_outlast_an_uninstall() {
    let sandbox = Sandbox::new();
    let custom = sandbox.home.join(".jarvis/custom");
    let installed = sandbox.home.join(".jarvis/standards");
    let packaged_review = fs::read_to_string(Path::new(STANDARDS).join("skills/review/SKILL.md")).unwrap();
    assert!(sandbox.jarvis(&["install", "--packaged"]).status.success());

    assert!(sandbox.jarvis(&["clone", "skills/review"]).status.success());
    let own_review = custom.join("skills/review/SKILL.md");
    assert_eq!(fs::read_to_string(&own_review).unwrap(), packaged_review);
    assert!(!sandbox.jarvis(&["clone", "skills/no-such-skill"]).status.success());

    fs::write(&own_review, "my own review\n").unwrap();
    fs::create_dir_all(custom.join("skills/mine")).unwrap();
    fs::write(custom.join("skills/mine/SKILL.md"), "a skill of mine\n").unwrap();
    fs::create_dir_all(custom.join("core")).unwrap();
    fs::write(custom.join("core/zz-mine.md"), "A rule of my own.\n").unwrap();
    assert!(sandbox.jarvis(&["clone", "skills/review"]).status.success());
    assert_eq!(fs::read_to_string(&own_review).unwrap(), "my own review\n");

    assert!(sandbox.jarvis(&["install"]).status.success());
    assert_eq!(
        fs::read_to_string(installed.join("skills/review/SKILL.md")).unwrap(),
        "my own review\n"
    );
    assert!(installed.join("skills/coding/SKILL.md").is_file());
    assert!(sandbox.home.join(".claude/skills/mine/SKILL.md").is_file());
    let session_start = sandbox.jarvis(&["hook", "claude", "SessionStart"]);
    assert!(String::from_utf8_lossy(&session_start.stdout).contains("A rule of my own."));

    fs::remove_dir_all(custom.join("skills/mine")).unwrap();
    assert!(sandbox.jarvis(&["install"]).status.success());
    assert!(!sandbox.home.join(".claude/skills/mine").exists());
    assert!(!sandbox
        .jarvis(&["install", "--standards", installed.to_str().unwrap()])
        .status
        .success());

    assert!(sandbox.jarvis(&["uninstall"]).status.success());
    assert!(!installed.exists());
    assert_eq!(fs::read_to_string(&own_review).unwrap(), "my own review\n");
}

#[test]
fn repo_init_only_trusts_a_repository_when_told_to_and_leaves_every_other_setting_alone() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let repo_folder = repo.to_str().unwrap();
    let claude_trust = sandbox.home.join(".claude.json");
    let codex_config = sandbox.home.join(".codex/config.toml");
    let repo_settings = repo.join(".claude/settings.json");
    let codex_config_before = "# mine\n[projects.\"/elsewhere\"]\ntrust_level = \"untrusted\"\n";
    fs::create_dir_all(repo_settings.parent().unwrap()).unwrap();
    fs::write(
        &claude_trust,
        r#"{"numStartups":3,"projects":{"/elsewhere":{"hasTrustDialogAccepted":false}}}"#,
    )
    .unwrap();
    fs::write(&codex_config, codex_config_before).unwrap();
    fs::write(
        &repo_settings,
        r#"{"permissions":{"allow":["Bash(ls *)","Bash(sed *)"],"deny":["Read(.env)"]}}"#,
    )
    .unwrap();

    let files = [&claude_trust, &codex_config, &repo_settings];
    let before_init = files.map(|file| fs::read(file).unwrap());
    assert!(!sandbox.jarvis_in(&repo, &["repo", "audit"]).status.success());
    assert!(!sandbox.jarvis_in(&repo, &["repo", "init"]).status.success());
    assert_eq!(files.map(|file| fs::read(file).unwrap()), before_init);
    assert!(sandbox.jarvis_in(&repo, &["repo", "init", "--yes"]).status.success());
    assert!(sandbox.jarvis_in(&repo, &["repo", "audit"]).status.success());

    let trust = read_json(&claude_trust);
    assert_eq!(trust["numStartups"], 3);
    assert_eq!(trust["projects"]["/elsewhere"]["hasTrustDialogAccepted"], false);
    assert_eq!(trust["projects"][repo_folder]["hasTrustDialogAccepted"], true);
    let settings = read_json(&repo_settings);
    let allowed = settings["permissions"]["allow"].as_array().unwrap();
    assert_eq!(allowed[0], "Bash(ls *)");
    assert_eq!(allowed.iter().filter(|rule| *rule == "Bash(sed *)").count(), 1);
    assert!(allowed.iter().any(|rule| rule == "Bash(git commit *)"));
    assert_eq!(settings["permissions"]["deny"][0], "Read(.env)");
    let codex = fs::read_to_string(&codex_config).unwrap();
    assert!(codex.starts_with(codex_config_before));
    assert!(codex.contains(&format!("[projects.\"{repo_folder}\"]\ntrust_level = \"trusted\"\n")));

    let after_the_first_init = files.map(|file| fs::read(file).unwrap());
    assert!(sandbox.jarvis_in(&repo, &["repo", "init"]).status.success());
    assert_eq!(files.map(|file| fs::read(file).unwrap()), after_the_first_init);
}

#[test]
fn codex_only_gets_a_run_that_can_fix_in_a_repository_it_trusts() {
    let sandbox = Sandbox::new();
    sandbox.install();
    let repo = sandbox.repo_with_one_pushed_commit();
    let codex = sandbox.fake_programs_dir().join("codex");
    let codex_args = sandbox.fake_programs_dir().join("codex-args");
    let codex_config = sandbox.home.join(".codex/config.toml");
    let trust_level = |level: &str| format!("[projects.\"{}\"]\ntrust_level = \"{level}\"\n", repo.display());
    fs::write(
        &codex,
        "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/codex-args\"\necho 'VERDICT: PASS'\n",
    )
    .unwrap();
    fs::set_permissions(&codex, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        sandbox.home.join(".jarvis/settings.toml"),
        "[review]\nagent = \"codex\"\nfix = true\n",
    )
    .unwrap();

    fs::write(&codex_config, trust_level("untrusted")).unwrap();
    assert!(!sandbox.jarvis_in(&repo, &["repo", "init", "--yes"]).status.success());
    assert_eq!(fs::read_to_string(&codex_config).unwrap(), trust_level("untrusted"));
    sandbox.commit(&repo, "lib.rs", "fn main() {}\n");
    assert!(sandbox.push(&repo).status.success());
    assert!(fs::read_to_string(&codex_args).unwrap().contains("--sandbox read-only"));
    assert_eq!(sandbox.fixing_recorded_for(&sandbox.head(&repo)), "read-only-agent");

    fs::write(&codex_config, trust_level("trusted")).unwrap();
    sandbox.commit(&repo, "more.rs", "fn more() {}\n");
    assert!(sandbox.push(&repo).status.success());
    assert!(!fs::read_to_string(&codex_args).unwrap().contains("--sandbox"));
}

#[test]
fn update_upgrades_through_homebrew_and_registers_the_path_that_outlasts_an_upgrade() {
    let sandbox = Sandbox::new();
    let homebrew = sandbox.home.join("homebrew");
    let keg = homebrew.join("Cellar/jarvis/1.0.0");
    let brew = homebrew.join("bin/brew");
    let lasting_jarvis = homebrew.join("opt/jarvis/bin/jarvis");
    for folder in [keg.join("bin"), homebrew.join("bin"), homebrew.join("opt")] {
        fs::create_dir_all(folder).unwrap();
    }
    fs::copy(env!("CARGO_BIN_EXE_jarvis"), keg.join("bin/jarvis")).unwrap();
    fs::write(keg.join("INSTALL_RECEIPT.json"), r#"{"source":{"tap":"someone/tap"}}"#).unwrap();
    symlink(&keg, homebrew.join("opt/jarvis")).unwrap();
    symlink(keg.join("bin/jarvis"), homebrew.join("bin/jarvis")).unwrap();
    fs::write(&brew, "#!/bin/sh\necho \"$@\" > \"${0%/*}/brew-args\"\n").unwrap();
    fs::set_permissions(&brew, fs::Permissions::from_mode(0o755)).unwrap();

    let mut update = Command::new(homebrew.join("bin/jarvis"));
    update.arg("--home").arg(&sandbox.home).arg("update");
    sandbox.with_only_the_fake_programs(&mut update);
    let updated = sandbox.with_only_the_sandbox_git_config(&mut update).output().unwrap();

    assert!(updated.status.success(), "{}", String::from_utf8_lossy(&updated.stderr));
    assert_eq!(
        fs::read_to_string(homebrew.join("bin/brew-args")).unwrap().trim(),
        "upgrade someone/tap/jarvis"
    );
    let registered = sandbox.git_output(&sandbox.home, &["config", "--global", "hook.jarvis.command"]);
    assert!(registered.contains(lasting_jarvis.to_str().unwrap()), "{registered}");
}
