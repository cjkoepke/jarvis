---
name: commenting
description: Use when writing or reviewing code comments and docblocks.
---

<!-- @TODO: sample skill that exercises skill delivery. Replace or remove it once real skills are written. -->

# Commenting

1. Delete any comment that describes your process, a plan, or what the code already says. Rename or extract until the code says it.
2. Add an API docblock only when the signature alone does not tell a caller what they need.
3. Put context a future agent needs in a docblock tagged `@agent`: a link to some other piece of context that informs the code.
4. Mark a stub or placeholder with a `@TODO` docblock that says when or why it will be removed and what it should be replaced with.
5. If you can give the same explanation without a comment, prefer no comment.