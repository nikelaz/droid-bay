# issue-research-gh

Generate a research report for a GitHub issue (grounded in the codebase, with web references), posted back to the issue as a comment.

## Usage

```sh
./issue-research-gh -owner nikelaz -repo droid-bay -issue 12
```

## Web search

Web search runs through SearXNG. By default the agent starts a SearXNG container with podman (docker as fallback) on a free localhost port and shuts it down on exit, so no API keys are needed. Set `SEARXNG_URL` to use an existing SearXNG instance instead, or `SEARXNG_IMAGE` to override the image (`docker.io/searxng/searxng:latest`).
