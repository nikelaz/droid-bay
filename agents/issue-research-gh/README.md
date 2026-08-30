# issue-research-gh

Generate a research report for a GitHub issue (grounded in the codebase, with web references), posted back to the issue as a comment.

## Usage

```sh
./issue-research-gh -owner nikelaz -repo droid-bay -issue 12
```

## Web search

Web search runs through SearXNG. The agent connects to an existing SearXNG instance at `http://127.0.0.1:8888`, or at `SEARXNG_URL` if set. If none is reachable, install SearXNG (https://docs.searxng.org) and run it locally.
