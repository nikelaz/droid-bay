# issue-to-plan-gh 

GitHub issue → implementation plan → posted back to the issue as a comment.

## Setup 

```sh
cp .env.example .env   # then fill in GITHUB_TOKEN and your provider API key
# variables already exported in your shell override the .env file

go build -o issue-to-plan-gh .
```

## Usage

```sh
./issue-to-plan-gh -owner nikelaz -repo droid-bay -issue 12
```

## Arguments:

| Flag      | Default     | Description                              |
| --------- | ----------- | ---------------------------------------- |
| `-owner`  | —           | Repository owner (user or org), required |
| `-repo`   | —           | Repository name, required                |
| `-issue`  | —           | Issue number, required                   |
