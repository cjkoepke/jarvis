# Working on jarvis

jarvis is a Rust command-line tool. It delivers one folder of standards to every
AI agent session and sub-agent on the machine, and has an agent review every
`git push` against the same standards. `README.md` says why; this file says how
it is built.

## Build and run

```sh
cargo build
cargo test
cargo clippy --all-targets
```

Run it against a throwaway home, never the real one:

```sh
mkdir -p /tmp/jarvis-home
target/debug/jarvis --home /tmp/jarvis-home install --standards ./standards
target/debug/jarvis --home /tmp/jarvis-home status
target/debug/jarvis --home /tmp/jarvis-home hook claude SessionStart
target/debug/jarvis --home /tmp/jarvis-home uninstall
```

Without `--home`, `install` and `uninstall` act on the real home directory:
`~/.claude/settings.json`, `~/.claude/skills`, `~/.codex/hooks.json`,
`~/.agents/skills`, `~/.gitconfig` and `~/.jarvis`. `repo init` writes
`~/.claude.json`, `~/.codex/config.toml` and the `.claude/settings.json` of the
repository it is run in, so try it in a throwaway repository as well.

`jarvis settings` needs a terminal, and so do the questions `repo init` and
`repo audit` ask, so drive them through a pseudo-terminal to try them from an
agent session.

## What each command does

- **`install`** copies the standards to `~/.jarvis/standards`, registers
  `SessionStart` and `SubagentStart` hooks for Claude Code and Codex, copies
  the skills to where each agent looks for them, and adds a `pre-push` hook to
  the global git config (Git 2.54 or newer). A bare `jarvis install` reuses the
  standards folder and agents of the last install. When that folder is gone it
  falls back to the `standards/` of the checkout the binary was built from,
  then to `./standards`.
- **`hook`** is what the agent hooks run. It replies with the core.
- **`pre-push`** is what the git hook runs. It starts the headless agent the
  settings name with the `review` skill. Claude Code may only read files and
  run `git log`, `git show`, `git diff` and `printenv`; Codex runs in its
  read-only sandbox. Violations refuse the push and are printed to the caller,
  so an agent that ran `git push` reads them as the command's output. A push
  is reviewed once: after a refusal the next attempt goes out without a review,
  and the push after that is reviewed again. With `review.fix` on, the
  reviewer fixes what it finds and commits the fixes on top, where the
  repository's agent settings let it; that push is still refused, and the next
  attempt sends them.
- **`status`** shows checks on the standards, each agent's hooks and skills,
  the git hook and the reviewer. `--format json` prints the same for an agent.
  It only reports, and each failed row says how to fix it.
- **`settings`** opens a keyboard-driven screen for the machine's defaults and
  saves each change as it is made.
- **`repo init`** readies the checkout it is run in for a fixing reviewer. For
  each installed agent, and for the reviewer when it is not one of them, it
  records that the repository is trusted. For Claude Code it also adds the
  [permission rules](#letting-the-reviewer-fix) to the repository's
  `.claude/settings.json`. Where an agent does not trust the repository yet,
  it asks first, and a no changes nothing. `--yes` answers for a run without a
  terminal, which otherwise stops there. Running it again changes nothing, and
  it fails when something is still missing afterwards.
- **`repo audit`** checks what `repo init` sets up: whether each of those
  agents trusts the repository, whether its rules allow editing and
  committing, and whether `review.fix` is on. It takes `--format json` and
  fails when a check does. In a terminal it then offers the fixes jarvis can
  make as a list to tick, the failed ones ticked already, and shows the checks
  again after making them. Without a terminal, and with `--format json`, it
  only reports.
- **`uninstall`** removes everything `install` added and nothing else. What
  `repo init` wrote stays: those are each agent's own settings for a repository.

## Rules that must hold

- `hook` and `pre-push` exit 0 on their own failures. jarvis is never the
  reason a session or a push breaks, and a review that cannot run lets the
  push through.
- `install` and `uninstall` leave configuration they did not write alone.
- Every path hangs off `Env::home`, so `--home` moves a whole install.
- Claude Code and Codex stay at parity. A capability added for one agent is
  added for the other in the same change.
- Nothing is ever written to the standards source folder. A machine that got
  jarvis as a binary does not have one.

## Why it is built this way

- **Core cap.** Claude Code drops hook context longer than 10,000 characters
  (https://code.claude.com/docs/en/hooks). The cap is 6,000 because a core
  that long stops being followed.
- **`SubagentStart` as well as `SessionStart`.** Claude Code's built-in
  Explore and Plan sub-agents skip `CLAUDE.md`, so a hook is the only way the
  standards reach them (https://code.claude.com/docs/en/sub-agents).
- **Codex approval.** Codex reads `~/.codex/hooks.json` in the shape Claude
  Code uses, but ignores a hook until the person approves it once
  (https://learn.chatgpt.com/docs/hooks). jarvis can only read that approval.
  The state key it looks up follows `hook_key` in
  https://github.com/openai/codex/blob/main/codex-rs/hooks/src/lib.rs.
- **Git hook in config, not `core.hooksPath`.** A hook defined in git config
  runs in addition to a repository's own hooks, so husky, lefthook and
  pre-commit keep working. `core.hooksPath` silently disables `.git/hooks`.
  Config hooks arrived in Git 2.54; older versions ignore the section.
- **Pre-push input and output.** Git gives the hook one line per ref on stdin:
  `<local ref> <local sha> <remote ref> <remote sha>`, where an all-zero sha
  means no commit. Results go to stderr, which git relays to whoever ran
  `git push`.
- **One review per push.** A refusal is recorded in
  `~/.jarvis/refused-pushes.json` as the remote URL and ref with the tip that
  ref had on the remote. The next push skips its review only while the remote
  ref still has that tip, and the record is dropped as soon as a push is let
  through. So the pass is spent once and cannot outlive a push, even one made
  from another clone.
- **Fixing reviewer.** A pre-push hook cannot change which commits its push
  sends, so a push the reviewer fixed is refused like any other and the fix
  rides on the one unreviewed push that follows. The reviewer commits on
  `HEAD`, so it only fixes a push of a single ref that moves with `HEAD`, the
  checked-out branch or `HEAD` itself, with no uncommitted edits to tracked
  files; any other push gets the read-only review. `VERDICT: FIXED` is
  only believed when `HEAD` gained commits on top of the pushed one.
- **Fixing permissions.** A review grants a fixing reviewer nothing. The
  permission to edit and commit comes from each agent's own settings for the
  repository, so each repository decides what its reviewer may do. Both agents
  only honour those once the person has trusted the repository in that agent.
  Where nothing grants the permission, every edit is denied and the reviewer
  reports its findings like a read-only one. `repo init` writes those settings,
  and only when a person runs it in the repository.
- **Requirements.** A `Requirement` pairs a check with the remedy jarvis has
  for it, if any. `repo init` applies the remedies of the failed checks and
  `repo audit` offers them, so the two can never disagree about what is
  missing or how it is put right.
- **Asking before trusting.** Trust lets a repository's own settings, hooks
  and tools run in every session of an agent there, so jarvis only records it
  on a yes: the answer to `repo init`, its `--yes`, or a ticked line in
  `repo audit`.
- **Repository trust.** Neither agent has a command that records trust without
  a session, so `repo init` writes what the agent's own prompt would. Both
  look the trust up under the exact folder: a trusted parent folder does not
  count, and a linked worktree takes the trust of the checkout it was added
  from. `Repo` works out both folders.
- **Trust, Claude Code.** It is `projects.<folder>.hasTrustDialogAccepted` in
  `~/.claude.json`
  (https://code.claude.com/docs/en/permissions). Without it Claude Code drops
  the `permissions.allow` of the repository's `.claude/settings.json`, also in
  a `--print` run. A running session saves that file under a lock, a folder
  named `.claude.json.lock`, and by renaming a finished copy over it. jarvis
  does the same, so neither loses the other's change.
- **Trust, Codex.** It is `trust_level = "trusted"` under
  `[projects."<folder>"]` in `~/.codex/config.toml`. Codex asks about the
  checkout first and the main checkout second, and the first that has an entry
  decides, even one without a trust level. A folder marked `untrusted` is the
  person's decision, which `repo init` leaves alone and `repo audit` reports.
  The file is edited with `toml_edit`, which keeps its comments and order, and
  no second header is ever added for a folder: a duplicate, or any other trust
  level, stops Codex from starting.
- **Fixing run, Claude Code.** It drops the read-only run's `--tools` limit.
  `--safe-mode` keeps `CLAUDE.md`, skills and hooks out of the review, but
  permission settings still apply and `--permission-prompts none` denies what
  they do not allow. Claude Code denies a `git` command that follows a `cd`,
  so `CLAUDE_BASH_MAINTAIN_PROJECT_WORKING_DIR` keeps the reviewer's shell in
  the repository.
- **Fixing run, Codex.** It drops the read-only run's `--sandbox read-only`,
  so Codex picks the sandbox, which is `workspace-write` in a repository the
  person trusted. Codex picks the same for a repository marked `untrusted`, so
  jarvis only starts this run where the repository is trusted and keeps the
  read-only one everywhere else.
  `workspace-write` keeps `.git` read-only, so the repository's git folders
  are passed with `--add-dir`; a linked worktree has two. A signing agent is
  reached over a socket, which the sandbox counts as network, so network
  access is switched on only when `commit.gpgsign` is set. Neither has any
  effect in a read-only sandbox.
- **Signing a fix.** A sandboxed reviewer can reach a running gpg-agent but
  cannot start one, so jarvis starts it before a fixing run when the
  repository signs commits with GPG. A locked key is then unlocked through the
  agent's own pinentry. A reviewer has no terminal, so a pinentry that needs
  one fails; the reviewer then undoes its edit and the push is refused for the
  pusher to fix.
- **`printenv` takes one name.** macOS `printenv` prints only the first
  variable it is given, so the review skill reads each on its own line.
- **Standards the binary was built from.** Cargo bakes the checkout path in at
  compile time. A binary that has left its checkout behind skips that folder
  like any other that is gone.
- **Hook files.** jarvis hooks are matched loosely, not on the exact command,
  so hooks written by a binary at an older path are still replaced. A hook file
  that is not a JSON object is an error: jarvis never overwrites configuration
  it could not read.
- **Skills.** Only skill folders carrying a `.jarvis` file are replaced or
  removed, which keeps a person's own skill with the same name safe.
- **Install order.** The standards source is loaded before anything is copied,
  so a broken folder never replaces a working install.
- **Settings screen.** `Screen` restores raw mode and line wrapping when
  dropped. Wrapping is off while it is open because a wrapped line would make
  the block taller than the redraw expects.
- **Tests.** `tests/e2e.rs` has its own sandbox because no crate gives a
  throwaway home together with a git whose global config lives in it. Every git
  call goes through `git()` so the real user and system config are never read.

## Settings

Settings belong to the machine, not to the standards. They live in
`~/.jarvis/settings.toml`, which `install` never replaces and `uninstall`
removes.

| Key | Holds |
| --- | --- |
| `review.agent` | The agent that reviews every push: `claude` (the default) or `codex`. |
| `review.model` | The model it reviews on. Without one, the agent's own default. |
| `review.fix` | Whether the reviewer fixes what it finds and commits it, in repositories that [let it](#letting-the-reviewer-fix). `false` by default. |

`jarvis settings` offers the models each agent shows in its own `/model`
picker. Both lists are read the same way, from the catalog the agent keeps on
disk for the signed-in account, so neither goes stale:

| Agent | Catalog | Offered |
| --- | --- | --- |
| Claude Code | newest `~/.claude/cache/model-catalog/*-cc.json` | models in the `main` section |
| Codex | `~/.codex/models_cache.json` | models with `visibility` `list`, by `priority` |

Without a readable catalog the list is empty and a name can still be typed.
Switching the reviewer drops the model, because a model name belongs to one
agent and would fail the other's run.

### Letting the reviewer fix

`review.fix` only asks for fixes. Each agent's own settings for a repository
allow them, and those apply to every session of that agent there, not only to
the review.

Claude Code needs the repository trusted and these rules in its
`.claude/settings.json`:

```json
{
  "permissions": {
    "allow": [
      "Edit(/**)",
      "Bash(sed *)",
      "Bash(git status *)",
      "Bash(git add *)",
      "Bash(git commit *)",
      "Bash(git restore *)"
    ]
  }
}
```

`Edit(/**)` covers every tool that writes files and stops at the folder the
reviewer starts in, which also bounds `sed -i`. Git starts a hook in the top
folder of the checkout, so that is the repository's edge.

Codex only needs the repository trusted.

`jarvis repo init` sets all of this up for the repository it is run in, and
`jarvis repo audit` shows what is missing. Claude Code reads the rules from the
checkout the push comes from, so other worktrees only get them once
`.claude/settings.json` is committed, and another clone needs its own
`repo init` as well. Where that file is tracked, commit the change `repo init`
made: the reviewer does not fix a push while tracked files have uncommitted
edits.

## Standards folder

| Folder | Used for |
| --- | --- |
| `core/*.md` | Always in context. Capped at 6,000 characters. |
| `skills/<name>/SKILL.md` | Loaded by the agent when a task needs it, and by the reviewer when it bears on a push. |
| `skills/review/SKILL.md` | How to review commits. Any agent can use it, and the pre-push reviewer runs it. |

After the source changes, `jarvis status` reports the installed copy as out of
date until the next `jarvis install`.

## Prompts

jarvis writes no prompt text of its own. Every agent gets the standards as they
are written:

- Session and sub-agent hooks send the core, with nothing around it.
- The pre-push reviewer gets the core followed by the `review` skill, as a
  session would after loading that skill. It gathers the commits, the diff and
  the other skills itself, with the commands the skill shows.

Anything a run needs that the standards cannot know reaches the agent as an
environment variable, and the skill shows how to read it. These are the ones
jarvis sets for the review:

| Variable | Holds |
| --- | --- |
| `JARVIS_PUSH_REMOTE` | The remote being pushed to. |
| `JARVIS_PUSH_COMMITS` | The tip of each pushed ref the remote lacks, separated by spaces. |
| `JARVIS_STANDARDS` | The installed standards folder, holding `core/` and `skills/`. |
| `JARVIS_REVIEW_FIX` | `true` when the reviewer may fix what it finds and commit, `false` otherwise. |

The review skill must ask for a last line of `VERDICT: PASS`, `VERDICT: FIXED`
or `VERDICT: FAIL`. jarvis reads only that line.

## Source layout

| Path | Holds |
| --- | --- |
| `src/cli.rs` | Command-line definition. |
| `src/commands/` | One file per command: `install`, `uninstall`, `status`, `settings`, `hook`, `pre_push`. `repo/` holds `init` and `audit`. |
| `src/agents/` | One file per agent, one per agent's model catalog and one per agent's settings for a repository, plus hook-file editing, skill copying and status checks. |
| `src/check/` | The check type, its table and JSON output, the report `status` and `repo audit` print, and a check paired with its remedy. |
| `src/ui/` | The colour palette and styled messages every command prints through. `prompt/` holds the keyboard-driven picker, checklist and text input. |
| `src/review/` | Running the reviewer, reading its verdict, counting its fix commits and remembering refused pushes. |
| `src/git/` | Git hook registration, finding the pushed commits the remote lacks, and what an agent working in the repository needs to know about it. |
| `src/standards.rs` | Loading the standards folder. |
| `src/settings.rs` | Reading and writing `~/.jarvis/settings.toml`. |
| `tests/e2e.rs` | Whole-command tests in a throwaway home with a fake reviewer. |

To support another agent, add a file in `src/agents/` that implements `Agent`
and list it in `all()`. Hook registration, skills, status checks, the review
and the `repo` commands need no changes.

To add a setting, add its field in `src/settings.rs` and a `Setting` variant in
`src/commands/settings.rs`.

## Not built yet

- The reviewer cannot fix a commit message: it only adds commits.
- `repo audit` compares rules by their spelling, so it asks for the ones
  `repo init` writes even where a broader rule already allows the same. It
  does not look for what would still stop a fix: a Claude Code `deny` rule, or
  a Codex `sandbox_mode` set in a config file.
- `CLAUDE_CONFIG_DIR` and `CODEX_HOME` are not followed. jarvis reads and
  writes each agent's files under the home folder only.
- No repo sync.
- Codex delivery only works once the hooks are approved with `/hooks`. Until
  then Codex skips them and sessions start without the standards.
- `status` cannot tell when Codex wants a changed hook approved again; it still
  reports the hook as approved.
