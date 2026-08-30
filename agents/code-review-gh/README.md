# code-review-gh

Code review with an LLM. Works on:
- GitHub pull requests and commits
- Local uncommited changes

## Usage

Pick exactly one target:

```sh
# Review a pull request (posts a review comment on the PR)
./code-review-gh -owner nikelaz -repo droid-bay -pr 12

# Review a single commit (posts a comment on the commit)
./code-review-gh -owner nikelaz -repo droid-bay -commit 4f2a9c1e

# Review local unstaged changes (prints the review to stdout)
./code-review-gh -path /path/to/working/tree
```

## Options

| Flag | Description |
| --- | --- |
| `-owner`, `-repo` | Repository owner and name; required with `-pr` and `-commit`. |
| `-pr <number>` | Review a GitHub pull request. |
| `-commit <sha>` | Review a single GitHub commit (full or abbreviated SHA). |
| `-path <dir>` | Review local unstaged changes (`git diff`) in a working tree. |
| `-model <name>` | Model to use; overrides the compiled-in model defaults and skips its reasoning effort. |
| `-codebase <path>` | Reuse a checkout already at the target ref instead of cloning (GitHub targets only). |
| `-post=false` | Don't post to GitHub; print the review to stdout instead (GitHub targets only). |

