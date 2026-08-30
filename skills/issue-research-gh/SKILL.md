---
name: issue-research-gh
description: LLM research report for a GitHub issue grounded in the codebase and web sources. Use when an issue needs investigation before work. Posts the report back to the issue as a comment.
---

```sh
agents/issue-research-gh/linux-amd64/issue-research-gh -owner <owner> -repo <repo> -issue <number>
```

The paths above are for linux-amd64, adjust for the platform you're running on.

By default the agent will post the research as a comment in the GitHub issue.
