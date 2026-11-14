# jarvis

A personal tool for better agent work. One folder of standards reaches every
AI agent session and sub-agent on the machine, and an agent reviews every
`git push` against the same standards.

This is a proof of concept. `standards/core/core.md` is the real Constitution;
the `commenting` skill is a sample marked `@TODO`.

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
  config (Git 2.54 or newer). It runs the headless agent that `settings.toml`
  names with the `review` skill, which finds the commits the remote lacks and
  the skills that bear on them itself. Claude Code may only read files and run
  `git log`, `git show`, `git diff` and `printenv`; Codex runs in its read-only
  sandbox. Violations refuse the push and are printed to the caller, so an agent that
  ran `git push` reads them as the command's output, fixes them and pushes
  again. A review that cannot run never stops a push.
- **Status.** `jarvis status` shows a table of checks on the standards, each
  agent's hooks and skills, the git hook and the reviewer. `--format json` prints the same for
  an agent. It only reports, and each failed row says how to fix it.

## Standards folder

| Folder | Used for |
| --- | --- |
| `core/*.md` | Always in context. Capped at 6,000 characters. |
| `skills/<name>/SKILL.md` | Loaded by the agent when a task needs it, and by the reviewer when it bears on a push. |
| `skills/review/SKILL.md` | How to review commits. Any agent can use it, and the pre-push reviewer runs it. |
| `settings.toml` | `[review]`: `agent` reviews pushes (`claude` or `codex`, default `claude`) on `model` (default: the agent's own). |

After changing the standards, `jarvis status` reports the installed copy as out
of date. A bare `jarvis install` refreshes it from any directory: it reuses the
standards folder and agents of the last install.

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

The review skill must ask for a last line of `VERDICT: PASS` or
`VERDICT: FAIL`. jarvis reads only that line.

## Source layout

| Path | Holds |
| --- | --- |
| `src/cli.rs` | Command-line definition. |
| `src/commands/` | One file per command: `install`, `uninstall`, `status`, `hook`, `pre_push`. |
| `src/agents/` | One file per agent, plus hook-file editing, skill copying and status checks. |
| `src/check/` | The check type and its table and JSON output. |
| `src/ui/` | The colour palette and styled messages every command prints through. |
| `src/review/` | Running the reviewer and reading its verdict. |
| `src/git/` | Git hook registration and finding the pushed commits the remote lacks. |
| `src/standards.rs` | Loading the standards folder. |
| `src/settings.rs` | Reading `settings.toml`. |

To support another agent, add a file in `src/agents/` that implements `Agent`
and list it in `all()`.

## Not built yet

- The reviewer only reports. It does not fix what it finds.
- No repo bootstrap or sync.
- Codex delivery only works once the hooks are approved with `/hooks`. Until
  then Codex skips them and sessions start without the standards.
- `status` cannot tell when Codex wants a changed hook approved again; it still
  reports the hook as approved.
