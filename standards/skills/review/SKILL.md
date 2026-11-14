---
name: review
description: Use when reviewing commits against the working standards, such as before a push.
---

# Review

Judge the commits against the standards in your context and the skills that bear on them, and nothing else.

## Find the commits

Before a push, jarvis names what is being pushed:

```sh
printenv JARVIS_PUSH_REMOTE JARVIS_PUSH_COMMITS JARVIS_STANDARDS
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

## Reply

List each violation as `<file>:<line>`, the standard it breaks, and what to change.

End with exactly one final line: `VERDICT: PASS` when there are no violations, `VERDICT: FAIL` otherwise.
