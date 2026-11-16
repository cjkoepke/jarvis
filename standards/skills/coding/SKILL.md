---
name: coding
description: use whenever you begin to write any sort of code
---

## Methodology

- _Always_ write your code in a way that a human engineer can understand easily.
- Ensure that your code concerns are separated. Make heavy use of modular design and compose the workspace filenames and folders in a way that makes it easy to scan sections of the entire codebase.
- For functions that take more than 3 arguments, prefer a struct, record, or an object with defaults so that call-sites easily describe the arguments purpose. For example, a function that reads `func(arg, arg2, arg3, arg4, arg5)` says nothing about what those values mean. Prefer instead: `func({ someIntent: arg, otherIntent: arg2, ...rest }).`

# Commenting

- _Never_ leave comments explaining your process, a plan, or what the code itself can explain.
- Extra context that an agent might need should be added to AGENTS.md only _after_ it has been considered in the context of the rest of the file.
- API docblocks are allowed for the purpose of providing additional context to IDEs, but only if the signature is insufficient.
- Mark a stub or placeholder with a `@TODO` docblock that says when or why it will be removed and what it should be replaced with.
- If you can give the same explanation without a comment, prefer no comment.

## Testing

- Write tests _only_ for the logic that you've introduced.
- _Never_ over-test: write only tests that safeguard critical logic or internal functions that have many dependents.
- _Never_ write tests that pin copy, this is a complete waste of time.
- _Never_ write tests that enforce a UI pattern, except for when that UI pattern employs critical logic, such as a swap ratio output.
- Only write test harnesses when a third-party testing library is insufficient.
- Never write a test that makes an api call, these should be mocked.
