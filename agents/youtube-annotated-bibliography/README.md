# youtube-annotated-bibliography

Generate an annotated bibliography of YouTube videos for a video idea stored as a Linear issue, then attach it to the issue as a Linear document named "LLM Generated Annotated Bibliography".

The agent takes the issue (title and description are the rough video idea), searches the web for real, existing YouTube videos about the topic, and annotates each one: what it covers, its key claims, and why it matters for the video.

## Usage

```sh
./youtube-annotated-bibliography -issue ENG-123
```

`-issue` accepts a Linear issue identifier (`ENG-123`), a UUID, or an issue URL. `-model` overrides `model-defaults.json` and skips its reasoning effort.

## Web search

Web search runs through SearXNG. The agent connects to an existing SearXNG instance at `http://127.0.0.1:8888`, or at `SEARXNG_URL` if set. If none is reachable, install SearXNG (https://docs.searxng.org) and run it locally.
