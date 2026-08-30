---
name: code-review-gh
description: LLM code review of a GitHub pull request, a single GitHub commit, or local unstaged git changes. Use when code should be reviewed before merge. Posts the review to GitHub by default.
---

You need to pick pr, commit or local uncommited changes review:

```sh
agents/code-review-gh/linux-amd64/code-review-gh -owner <owner> -repo <repo> -pr <number>
agents/code-review-gh/linux-amd64/code-review-gh -owner <owner> -repo <repo> -commit <sha>
agents/code-review-gh/linux-amd64/code-review-gh -path <working-tree>
```

The paths above are for linux-amd64, adjust for the platform you're running on.

By default the agent will post the review as a comment in GitHub (if working on a github repo).
The `-post=false` option prints instead of posting a comment in GitHub.
