---

name: GitHub
description: Rules for interacting with GitHub, including reviews, pull requests, commit styles, etc.
---

## Pull Requests
1. Keep your PR descriptions short and to the point.
2. Always descibe the *intent* of a PR, not its technical output (this is evident in the code review).
3. Always sign off as "_Generated with Jarvis_", _never_ include "Generated with Claude Code".
4. Follow the repositories template pattern if there is one, otherwise fallback to this template:
    1. Problem: describe the problem briefly that prompted opening a PR.
    2. Solution: describe the overarching way that this PR solves the stated problem.
    3. Verification: describe what you did to verify your solution solves the problem.

## Commits
1. Use standard git plus the repository forge CLI for pull request or merge request operations using GitHub CLI `gh`.
2. Inspect the current repo and existing pull request or merge request state before choosing commit messages, titles, bodies, or replies.
3. Create meaningful commit boundaries: split unrelated changes into separate commits and keep each commit focused.
4. Use Conventional Commits for all new commits (`type(scope): summary` or `type: summary`; types: feat, fix, chore, docs, refactor, test, perf, build, ci, style).
5. Avoid overly verbose commit messages, and rely on the code to explain itself.
6. For multi-line pull request or merge request content, write it to a temp file or heredoc and pass the file to the forge CLI instead of inline multi-line body text.
7. If a command fails, resolve the issue and retry rather than stopping early.

## Reviews
1. When responding to a review, independantly weigh the input.
2. Always read the entire review comments before beginning to fix.
3. Group fixes by related context into their own commits.
4. When you are done, leave a summary comment on the PR describing what you did and what you ignored.
5. Resolve all review comments that you addressed.
6. Resolve a comment without a response, unless the summary comment doesn't address it.