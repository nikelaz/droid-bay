# youtube-annotated-bibliography

Generate an annotated bibliography of YouTube videos for a video idea stored as a Linear issue, then attach it to the issue as a Linear document named "LLM Generated Annotated Bibliography".

The agent takes the issue (title and description are the rough video idea), searches the web for real, existing YouTube videos about the topic, and annotates each one: what it covers, its key claims, and why it matters for the video.

## Usage

```sh
./youtube-annotated-bibliography -issue ENG-123
```

`-issue` accepts a Linear issue identifier (`ENG-123`), a UUID, or an issue URL. `-model` overrides `model-defaults.json` and skips its reasoning effort.

## Web search

Web search runs through SearXNG. By default the agent starts a SearXNG container with podman (docker as fallback) on a free localhost port and shuts it down on exit, so no API keys are needed. Set `SEARXNG_URL` to use an existing SearXNG instance instead, or `SEARXNG_IMAGE` to override the image (`docker.io/searxng/searxng:latest`).
