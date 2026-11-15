# jarvis

Your working standards in every AI agent session, and a review of every push
against them.

## Why

Agents do better work when they know how you work. Today that knowledge is
scattered: each tool has its own config file, sub-agents start without it, and
nothing checks whether the result followed it. jarvis keeps your standards in
one folder and makes them count everywhere.

## What you get

- **One source of truth.** Write your standards once. Every Claude Code and
  Codex session, and every sub-agent they start, begins with them.
- **Skills on demand.** Longer procedures stay out of the context until a task
  needs them.
- **A review before every push.** An agent reads your outgoing commits against
  the same standards. If something breaks them, the push is refused and the
  agent that pushed reads why, fixes it and pushes again.
- **Out of your way.** Launch agents however you like. A review that cannot run
  never blocks a push, and `uninstall` puts everything back as it was.

## Get started

```sh
cargo install --path .
jarvis install --standards ./standards
```

| Command | Does |
| --- | --- |
| `jarvis install` | Sets up your agents and the push review from a standards folder. |
| `jarvis status` | Shows what is in place and how to fix what is not. |
| `jarvis settings` | Picks the reviewer and its model with the arrow keys. |
| `jarvis uninstall` | Removes everything `install` added and nothing else. |

It needs Git 2.54 or newer and Claude Code or Codex. Codex asks you to approve
the hooks once with `/hooks`.

## Your standards

A standards folder has two parts:

- `core/` is short and always in context.
- `skills/` holds one folder per procedure, loaded when a task calls for it.

The `standards/` folder in this repository is a working example to start from.

## Status

jarvis is an early proof of concept. The reviewer reports what it finds and
does not fix it yet.

Working on jarvis itself? See [AGENTS.md](AGENTS.md).
