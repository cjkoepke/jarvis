---
name: review
description: Use when reviewing commits against the working standards, such as before a push.
---

# Review

Judge the commits against the standards in your context and the skills that bear on them, and nothing else.

## Find the commits

Before a push, jarvis names what is being pushed:

```sh
printenv JARVIS_PUSH_REMOTE
printenv JARVIS_PUSH_COMMITS
printenv JARVIS_STANDARDS
```

With those values, read each commit's message and change:

```sh
git log -p <JARVIS_PUSH_COMMITS> --not --remotes=<JARVIS_PUSH_REMOTE>
```

When the variables are not set, review what the current branch has that no remote has:

```sh
git log -p HEAD --not --remotes
```

For more context, read files in the working tree or run `git show <commit>:<path>`. The working tree can hold uncommitted edits, so judge the commits, not the files on disk.

## Find the skills

Read every skill whose description bears on the commits. Before a push they are in `<JARVIS_STANDARDS>/skills/`; otherwise they are the skills installed for you.

## Judge

1. Review only what the commits add or change, including their messages. Leave alone anything they did not touch.
2. Report only violations you are sure of.
3. Do not praise, summarise, or suggest improvements the standards do not ask for.
4. Include a code quality coverage review and possible areas to refine, consolidate, or generally simplify on the assumption that you are a better model than whatever made the changes.

## Fix

Before a push, jarvis says whether you may fix what you find:

```sh
printenv JARVIS_REVIEW_FIX
```

When it prints `true`, fix every violation you can. You may edit files in the repository with your file-editing tool or `sed`, and run `git status`, `git add`, `git commit` and `git restore`. Expect anything else to be denied, such as a `git` command that follows a `cd`.

1. Edit the files, then commit the fixes the way the standards ask of any commit.
2. Stage only the files you changed, by name.
3. Only add commits. Never amend, rebase or push, and never skip signing or hooks. A violation that only a rewritten commit can fix, such as a commit message, stays unfixed.
4. Leave no uncommitted edit behind. Undo one you cannot commit with `git restore`.

Otherwise change nothing.

## Reply

List each violation as `<file>:<line>`, the standard it breaks, and what to change. For one you fixed, say what you changed instead.

End with exactly one final line:

- `VERDICT: PASS` when there are no violations.
- `VERDICT: FIXED` when you fixed and committed every violation.
- `VERDICT: FAIL` otherwise.
