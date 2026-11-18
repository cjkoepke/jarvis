# jarvis

**Models change. Your standards should not.**

jarvis is an open source, opinionated harness for AI coding agents: one set of
working standards in every Claude Code and Codex session, and a review of your
commits against those standards when you push, whichever model wrote the code.

## The problem

The model behind your agent keeps changing: a new release, a new default,
another provider. If the quality of your code changes with it, your standards
are whatever that model happens to do. Writing rules down does not fix that:

- **They are tied to a tool.** Each agent reads its own config file, so rules
  written for one are missing in the next.
- **Sub-agents can start without them.** Claude Code's built-in Explore and Plan
  sub-agents [skip `CLAUDE.md`](https://code.claude.com/docs/en/sub-agents).
- **Nothing checks the result.** A rule in a prompt is a request. Whether the
  commit followed it is left to the model that wrote it.

## What jarvis does about it

A model is a part you swap. Standards are what you keep. So jarvis keeps them
outside the model and checks the work against them:

- **Plain files.** Your standards are Markdown in one folder, outside any one
  tool's config.
- **The same start for every session.** When a session or a sub-agent starts, a
  hook hands Claude Code and Codex the same core: the short part that is always
  in context. Both get the same skills for the rest, whichever model they run.
- **A review at `git push`.** An agent reads your outgoing commits against the
  same standards, and a violation refuses the push, whatever wrote them: Claude
  Code, Codex, another tool or you.
- **A reviewer you choose.** Claude Code by default, or Codex, on any model name
  the agent accepts, so one can review what the other wrote. jarvis holds no
  API key: a review is a run of the agent CLI you are already signed in to.

That is the whole harness: jarvis does not launch, wrap or sandbox your
sessions. Two agents are supported today, **Claude Code** and **Codex**, as
equals, on whatever models those two run. A third takes Rust code, not
configuration.

## The opinions

jarvis has opinions of two kinds. Five are built into the tool:

- **Check the work.** Your commits are reviewed against the standards when you
  push, instead of hoping the model complied.
- **Keep the always-on part short.** The core is capped at 6,000 characters,
  because a longer one stops being followed.
- **Load the rest on demand.** Procedures are skills, which stay out of context
  until a task calls for them.
- **Add nothing.** Agents get your standards as written, with no prompt text of
  jarvis's own.
- **Break nothing.** jarvis's own failures never break a session or block a
  push, and it leaves alone the configuration it did not write and the
  programs it did not install.

The rest are the packaged standards, one always-on core and four skills:

| Piece | What it holds an agent to |
| --- | --- |
| [`core/core.md`](standards/core/core.md) | **How it talks to you.** Short, signal over noise. Ask a question only when the answer improves the outcome by a substantial margin. Make a long answer easy to scan. State intent, not a list of what was done. |
| [`skills/coding`](standards/skills/coding/SKILL.md) | **Code a human engineer understands easily.** Separate concerns. Past three arguments, prefer a struct, record or object. No comment that says what the code already says. Test only the logic you introduced, never pin copy, mock API calls. Search code with ast-grep. |
| [`skills/github`](standards/skills/github/SKILL.md) | **Intent, not output.** A pull request describes its intent and opens as a draft. Conventional Commits, one focused change each, terse messages. No commit before you have reviewed the work, no pull request unless you ask. |
| [`skills/orchestration`](standards/skills/orchestration/SKILL.md) | **Your token limits come first.** Offer cost tiers before starting sub-agents, start them only for a short list of jobs, and review their code before accepting it. |
| [`skills/review`](standards/skills/review/SKILL.md) | **Only violations the reviewer is sure of.** It judges what the commits add or change, messages included, against the standards and nothing else. No praise, no summary. Code-quality remarks are notes, which never refuse a push. |

This is one engineer's way of working, not an industry ruleset. Nothing in it
is tied to a language or a framework, and some of it is plainly personal, such
as signing pull requests "Generated with Jarvis". Keep what you agree with and
[replace the rest](#make-the-standards-yours).

## How it works

`jarvis install` sets up the session hooks, the skills and one git hook, once
for the whole machine. After that, a push has its new commits reviewed by a
headless agent, read-only unless you let it fix:

| The reviewer | What happens |
| --- | --- |
| finds no violations | The push goes out. |
| finds violations | The push is refused. The findings print as the output of `git push`: file, line, the standard broken and what to change. An agent that pushed reads them and can fix, commit and push again. That push goes out without another review. The refusal names the folder in `~/.jarvis/reviews` where the review is kept. |
| finds violations, and you let it fix | It commits the fixes on top of your branch. That push is still refused, and the next one sends them. Off by default: turn on `review.fix` and run `jarvis repo init` in the repository. |
| cannot run | The push goes out. A missing agent CLI, an error or a review that passes 10 minutes never blocks you. |

## Quick start

You need:

- **Git 2.54 or newer.** Older Git ignores the hook, so pushes go out
  unreviewed. `install` does not check for it; `jarvis status` does.
- **Claude Code (`claude`) or Codex (`codex`)** on your `PATH`, signed in.
- **A Rust toolchain on a Unix-like system.** jarvis is at version 0.0.1 with
  no release build yet, so you build it from source.

The packaged coding skill searches code with
[ast-grep](https://ast-grep.github.io). When `ast-grep` is not on your `PATH`,
`install` installs it with Homebrew, or with cargo where you have no Homebrew,
which compiles it for a few minutes.

```sh
git clone https://github.com/cjkoepke/jarvis.git
cd jarvis
cargo install --locked --path .
jarvis install   # machine-wide: standards, agent hooks, skills, pre-push review, ast-grep
jarvis status    # what is in place, and how to fix what is not
```

Then work as before: your next session starts with the core, and your next push
is reviewed. If you use Codex, do two things first:

- **Approve the hooks** once inside Codex with `/hooks`. Until then its sessions
  start without the standards.
- **Without Claude Code, make Codex the reviewer** in `jarvis settings`. Claude
  Code is the default, and without it pushes go out unreviewed.

## Commands

| Command | What it does |
| --- | --- |
| `jarvis install` | Copy the standards, register the agent hooks and the git pre-push review, and install ast-grep if it is missing. Run it again after your standards change: a bare run reuses the standards folder and agents of the last install, and the first uses the packaged standards and both agents. |
| `jarvis install --standards <folder>` | Use a folder holding `core/` and `skills/` in place of the packaged standards. |
| `jarvis install --packaged` | Go back to the standards packaged with jarvis after an install from a folder. Cannot be combined with `--standards`. |
| `jarvis install --agents <list>` | Set up only the agents you list, comma-separated: `claude`, `codex`. One you leave out loses its jarvis hooks and skills. |
| `jarvis status` | Show whether the standards, agent hooks, skills, git review and ast-grep are in place. It changes nothing, each failed row says how to fix it, and it exits non-zero when a check fails. `--format json` prints the same for an agent. |
| `jarvis clone [PIECES]...` | Copy core files or skills from the standards in effect into `~/.jarvis/custom`, to change them there. Name them, such as `skills/review` or `core/core.md`, or tick them from a list. |
| `jarvis settings` | Choose defaults such as the pre-push reviewer with the arrow keys: the reviewer, its model, and whether it fixes what it finds. |
| `jarvis repo init` | Trust this repository in each agent and give a fixing reviewer the permissions it needs. It asks first, unless you pass `--yes`. Both apply to every session of those agents there, not only the review. |
| `jarvis repo audit` | Show whether each agent lets a fixing reviewer edit and commit in this repository, and offer to fix what is missing. Takes `--format json` too, and exits non-zero when a check fails. |
| `jarvis update` | Bring jarvis up to date with the tool that installed it, then set everything up again. Today that tool is cargo, and a checkout is rebuilt as it is, so pull there first. |
| `jarvis uninstall` | Remove jarvis's hooks, skills and git hook, and its own data in `~/.jarvis`. Your own configuration, `~/.jarvis/custom`, what `repo init` wrote and the binary stay. ast-grep goes only when `install` put it there, with the package manager that did. |
| `--home <dir>` | On any command: the home directory to install into and read from, `$HOME` by default. An existing throwaway folder shows everything an install writes, but a missing ast-grep is still installed for the whole machine. |

`jarvis settings` writes `~/.jarvis/settings.toml`. You can edit it by hand:

```toml
[review]
agent = "codex"     # or "claude", the default
# model = "<name>"  # optional: a model name that agent accepts
fix = false         # true asks the reviewer to commit its fixes
```

`jarvis help`, `--help` and `--version` work as usual. `hook` and `pre-push` are
hidden: the agent hooks and git run them. The interactive screens need a
terminal, so a script or an agent names the pieces for `clone`, passes `--yes`
to `repo init`, reads `--format json` and edits `settings.toml`.

## Make the standards yours

Disagree with a packaged opinion? Replace it. A standards folder has two parts,
and your own live in `~/.jarvis/custom`, laid over the packaged ones:

- `core/*.md` is always in context, and capped at 6,000 characters in total.
- `skills/<name>/SKILL.md` is one folder per skill, in the
  [Agent Skills](https://github.com/agentskills/agentskills) format.

```sh
jarvis clone skills/coding   # copy it to ~/.jarvis/custom, then change it there
jarvis install               # deliver it to your agents and the reviewer
```

- **One piece at a time.** A core file or a skill with a packaged name replaces
  it, and a new name is added. You cannot remove a packaged piece.
- **Yours stays yours.** No command changes or removes a file there, and it
  outlasts an uninstall. What you have not cloned keeps changing with jarvis.
- **The review is a skill too.** Replace `skills/review` and a push is judged
  your way. Keep the final `VERDICT` line it asks for: jarvis reads only that.
- **Or bring a whole set.** `jarvis install --standards <folder>` replaces the
  packaged standards. Without a `review` skill, pushes go out unreviewed.

## What it is not

- **Not a gate.** A local git hook, not CI or branch protection. A review that
  cannot run lets the push through, and the push after a refusal is unreviewed.
- **Not a correctness check.** The review holds commits to written standards.
  It runs no tests, linters or builds, and it is no bug or security scanner.
- **Not free or instant.** Each reviewed push is a headless agent run on your
  own agent plan, and the push waits for it: a few minutes, 10 at most.
- **Not per repository.** One set of standards and one reviewer serve every
  repository you push from. `hook.jarvis.enabled = false` in your global git
  config switches the review off.
- **Not proof.** Nothing here measures a quality gain. You get the mechanism:
  the same standards in, a check on the way out.

## Contributing

Working on jarvis itself? [AGENTS.md](AGENTS.md) says how it is built, which
rules must hold and what is [not built yet](AGENTS.md#not-built-yet).

## License

[MIT](LICENSE)

