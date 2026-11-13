# jarvis

A personal tool for better agent work. One folder of standards reaches every
AI agent session and sub-agent on the machine, and a review before every
`git push` fixes what the context missed.

This is a proof of concept. `standards/core/core.md` is the real Constitution;
the skill and the rules are samples marked `@TODO`.

## Try it in a throwaway home

```sh
cargo build
mkdir -p /tmp/jarvis-home
target/debug/jarvis --home /tmp/jarvis-home install
target/debug/jarvis --home /tmp/jarvis-home status
target/debug/jarvis --home /tmp/jarvis-home hook claude SessionStart
target/debug/jarvis --home /tmp/jarvis-home uninstall
```

Without `--home`, `install` and `uninstall` act on your real home directory:
`~/.claude/settings.json`, `~/.claude/skills`, `~/.codex/hooks.json`,
`~/.agents/skills` and `~/.gitconfig`.

## What it does

- **Context.** `install` registers `SessionStart` and `SubagentStart` hooks for
  Claude Code and Codex. Each hook runs `jarvis hook`, which replies with
  `standards/core/`. Skills in `standards/skills/` are copied to where each
  agent looks for them. Codex needs its hooks approved once with `/hooks`.
- **Review before push.** `install` adds a `pre-push` hook to the global git
  config (Git 2.54 or newer). It checks the outgoing commits against
  `standards/rules/`. A `block` finding refuses the push. `fix` findings are
  corrected in one additional commit; git cannot add a commit to a push that
  is already under way, so that push stops and the next one sends it. `report`
  findings are printed and the push goes ahead. The result is printed to the
  caller, so an agent that ran `git push` reads it as the command's output.
- **Status.** `jarvis status` shows a table of checks on the standards, each
  agent's hooks and skills, and the git hook. `--format json` prints the same for
  an agent. It only reports, and each failed row says how to fix it.

## Standards folder

| Folder | Used for |
| --- | --- |
| `core/*.md` | Always in context. Capped at 6,000 characters. |
| `skills/<name>/SKILL.md` | Loaded by the agent when a task needs it. |
| `rules/*.toml` | Checked before push. `level` is `report`, `fix` or `block`. |

Rule kinds: `forbidden-path`, `forbidden-content` (added lines only),
`final-newline` and `trailing-whitespace` (the two the review can fix).

After changing the standards, `jarvis status` reports the installed copy as out
of date. A bare `jarvis install` refreshes it from any directory: it reuses the
standards folder and agents of the last install.

## Source layout

| Path | Holds |
| --- | --- |
| `src/cli.rs` | Command-line definition. |
| `src/commands/` | One file per command: `install`, `uninstall`, `status`, `hook`, `pre_push`. |
| `src/agents/` | One file per agent, plus hook-file editing, skill copying and status checks. |
| `src/check/` | The check type and its table and JSON output. |
| `src/ui/` | The colour palette and styled messages commands print through. |
| `src/rules/` | Rule loading, evaluation and fixes. |
| `src/git/` | Git hook registration, outgoing-commit inspection and the fix commit. |
| `src/standards.rs` | Loading `core/`, `skills/` and `rules/`, and the context text. |

To support another agent, add a file in `src/agents/` that implements `Agent`
and list it in `all()`.

## Not built yet

- The review only runs rules a program can decide. No model reads the change.
- No repo bootstrap or sync.
- Codex delivery only works once the hooks are approved with `/hooks`. Until
  then Codex skips them and sessions start without the standards.
