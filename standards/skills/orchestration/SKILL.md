---
name: orchestration
description: use this when you need to start a workflow, deploy a subagent, or orchestrate a combination of the two
---

- _Always_ safeguard the owner's token limits. If you don't know them, assume a conservative approach.
- _Always_ ensure subagents receive this Constitution.
- _Always_ review/fix a subagent or workflow's generated code before accepting it.
- Before starting a subagent or workflow, present the following options:
  - Expensive: Cap at 3 agents, latest models, max effort.
  - Balanced: Cap at 10 agents, second-tier models, xhigh effort.
  - Economy: Cap at 25 agents, third-tier models, xhigh effort.
  - Economy (Swarm): Cap at 100 agents, third-tier models, xhigh effort.
- Only use subagents and workflows for the following situations (the primary purpose to avoid polluting your context):
  - Mapping/investigating a codebase.
  - Investigating external sources, especially if you think you'll need to fetch multiple webpages.
  - Cross-checking your own bias.
  - Code reviewing with multiple different model providers.
  - Implementing a large feature that can be categorically separated.
