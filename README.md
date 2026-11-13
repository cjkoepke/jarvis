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
  config (Git 2.54 or newer). It hands the commits the remote lacks, their
  diff, the whole core and the list of skills to the headless agent that
  `settings.toml` names. The reviewer can read the working tree and the skill files and cannot change anything.
  Violations refuse the push and are printed to the caller, so an agent that
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
| `prompts/*.md` | Every prompt jarvis sends to an agent. See [Prompts](#prompts). |
| `settings.toml` | `[review]`: `agent` reviews pushes (`claude` or `codex`, default `claude`) on `model` (default: the agent's own). |

After changing the standards, `jarvis status` reports the installed copy as out
of date. A bare `jarvis install` refreshes it from any directory: it reuses the
standards folder and agents of the last install.

## Prompts

A prompt is always a markdown file in `prompts/`. Code reads the file and
replaces its `{{variable}}` placeholders, and never writes prompt text of its
own. Each prompt can use only the variables listed for it; `install` refuses
one that uses anything else. A new prompt gets its own file and its own row
here.

| File | Sent | Variables |
| --- | --- | --- |
| `review.md` | To the reviewer before each push. | `core`, `skills`, `commits`, `diff`, `pass`, `fail` |
| `session.md` | At each session start. | `core`, `skills` |
| `subagent.md` | At each sub-agent start. | `core` |

| Variable | Holds |
| --- | --- |
| `core` | The core files, joined. |
| `skills` | In `session.md`, the skill names. In `review.md`, each skill's name, description and file. |
| `commits` | The messages of the commits the remote lacks. |
| `diff` | Their diff. |
| `pass`, `fail` | The verdict line the reviewer must end with. |

Write the hook prompts as statements of fact. Claude Code can treat injected
commands as a prompt injection ([hooks](https://code.claude.com/docs/en/hooks)).

## Source layout

| Path | Holds |
| --- | --- |
| `src/cli.rs` | Command-line definition. |
| `src/commands/` | One file per command: `install`, `uninstall`, `status`, `hook`, `pre_push`. |
| `src/agents/` | One file per agent, plus hook-file editing, skill copying and status checks. |
| `src/check/` | The check type and its table and JSON output. |
| `src/ui/` | The colour palette and styled messages every command prints through. |
| `src/review/` | The review prompt's variables, running the reviewer and reading its verdict. |
| `src/git/` | Git hook registration and outgoing-commit inspection. |
| `src/standards.rs` | Loading the standards folder, and the hook context. |
| `src/prompts.rs` | Loading prompt files and filling in their variables. |
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
