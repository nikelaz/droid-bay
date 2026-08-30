# issue-research-gh

Generate a research report for a GitHub issue (grounded in the codebase, with web references), posted back to the issue as a comment.

## Setup

```sh
cp .env.example .env # and fill up the environment variables and keys needed
go build -o issue-research-gh .
```

## Usage

```sh
./issue-research-gh -owner nikelaz -repo droid-bay -issue 12
```
