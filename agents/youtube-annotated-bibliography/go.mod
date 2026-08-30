module github.com/nikelaz/droid-bay/agents/youtube-annotated-bibliography

go 1.26.7

require (
	github.com/nikelaz/droid-bay/helpers v0.0.0-00010101000000-000000000000
	github.com/nikelaz/droid-bay/sdk v0.0.0
)

require (
	github.com/zendev-sh/goai v0.9.8 // indirect
	golang.org/x/oauth2 v0.36.0 // indirect
)

replace github.com/nikelaz/droid-bay/sdk => ../../sdk

replace github.com/nikelaz/droid-bay/helpers => ../../helpers
