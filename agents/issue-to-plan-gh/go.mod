module github.com/nikelaz/droid-bay/agents/issue-to-plan-gh

go 1.27.0

require (
	github.com/nikelaz/droid-bay/sdk v0.0.0
	github.com/zendev-sh/goai v0.9.8
)

require golang.org/x/oauth2 v0.36.0 // indirect

replace github.com/nikelaz/droid-bay/sdk => ../../sdk
