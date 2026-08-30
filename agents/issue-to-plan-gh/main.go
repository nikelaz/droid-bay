package main

import (
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"log"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/nikelaz/droid-bay/sdk"
	"github.com/zendev-sh/goai"
	"github.com/zendev-sh/goai/mcp"
)

const PROMPT_PATH = "./prompt.md"
const SYSTEM_PROMPT = "You are a senior software engineer. Given a GitHub issue, produce a concise, actionable implementation plan in Markdown."

type User struct {
	Login string `json:"login"`
}

type Labels struct {
	Name string `json:"name"`
}

type Milestone struct {
	Title string `json:"title"`
}

type Issue struct {
	Number      int        `json:"number"`
	Title       string     `json:"title"`
	Body        string     `json:"body"`
	State       string     `json:"state"`
	HTMLURL     string     `json:"html_url"`
	CreatedAt   string     `json:"created_at"`
	UpdatedAt   string     `json:"updated_at"`
	User        User       `json:"user"`
	Labels      []Labels   `json:"labels"`
	Milestone   *Milestone `json:"milestone"`
	PullRequest *struct{}  `json:"pull_request"`
}

func main() {
	owner := flag.String("owner", "", "repository owner (user or org)")
	repo := flag.String("repo", "", "repository name")
	num := flag.Int("issue", 0, "issue number")
	envFile := flag.String("env", ".env", "path to an environment file (optional)")
	flag.Parse()

	if err := loadEnv(*envFile); err != nil {
		log.Fatalf("load env file: %v", err)
	}

	if *owner == "" || *repo == "" || *num <= 0 {
		log.Fatal("usage: issue-to-plan-gh -owner <owner> -repo <repo> -issue <number> [-env <file>]")
	}

	if os.Getenv("GITHUB_TOKEN") == "" {
		log.Fatal("GITHUB_TOKEN environment variable is required")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Minute)
	defer cancel()
	client := connectMCP(ctx)
	defer client.Close()

	readTool, commentTool, err := resolveTools(ctx, client)
	if err != nil {
		log.Fatal(err)
	}

	issue, err := getIssue(ctx, client, readTool, *owner, *repo, *num)
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("issue #%d: %s (state=%s)\n", issue.Number, issue.Title, issue.State)

	prompt, err := renderPrompt(issue)
	if err != nil {
		log.Fatal(err)
	}

	plan, err := sdk.Generate(ctx, llmConfig(), SYSTEM_PROMPT, prompt,
		goai.WithTemperature(0.2),
	)

	if err != nil {
		log.Fatal(err)
	}

	if strings.TrimSpace(plan) == "" {
		log.Fatal("LLM returned an empty plan; nothing to post")
	}

	fmt.Printf("\n--- implementation plan (%d bytes) ---\n%s\n", len(plan), plan)

	if err := postComment(ctx, client, commentTool, *owner, *repo, *num, plan); err != nil {
		log.Fatal(err)
	}

	fmt.Printf("\nplan posted to %s/%s#%d\n", *owner, *repo, *num)
}

func connectMCP(ctx context.Context) *mcp.Client {
	argv := strings.Fields(envOr("GITHUB_MCP_CMD", "npx -y @modelcontextprotocol/server-github"))

	transport := mcp.NewStdioTransport(argv[0], argv[1:], mcp.WithStdioEnv(map[string]string{
		"GITHUB_TOKEN":                 os.Getenv("GITHUB_TOKEN"),
		"GITHUB_PERSONAL_ACCESS_TOKEN": os.Getenv("GITHUB_TOKEN"),
	}))

	client := mcp.NewClient("issue-to-plan-gh", "1.0.0", mcp.WithTransport(transport),
		mcp.WithRequestTimeout(2*time.Minute))

	if err := client.Connect(ctx); err != nil {
		log.Fatalf("connect to GitHub MCP server: %v (is it installed / Node available?)", err)
	}

	return client
}

func resolveTools(ctx context.Context, client *mcp.Client) (readTool, commentTool string, err error) {
	res, err := client.ListTools(ctx, nil)
	if err != nil {
		return "", "", fmt.Errorf("list tools: %w", err)
	}

	for _, t := range res.Tools {
		switch {
		case t.Name == "issue_read" || t.Name == "get_issue":
			readTool = t.Name
		case t.Name == "add_issue_comment":
			commentTool = t.Name
		}
	}

	if readTool == "" {
		return "", "", fmt.Errorf("no issue-read tool found (issue_read/get_issue)")
	}

	if commentTool == "" {
		return "", "", fmt.Errorf("no add_issue_comment tool found")
	}

	return readTool, commentTool, nil
}

func getIssue(ctx context.Context, client *mcp.Client, tool, owner, repo string, num int) (*Issue, error) {
	args := map[string]any{"owner": owner, "repo": repo, "issue_number": num}

	if tool == "issue_read" {
		args["method"] = "get"
	}
	res, err := client.CallTool(ctx, tool, args)

	if err != nil {
		return nil, fmt.Errorf("%s via MCP: %w", tool, err)
	}

	if res.IsError {
		return nil, fmt.Errorf("%s via MCP: %s", tool, mcp.FormatContent(res.Content, true))
	}

	var issue Issue
	if err := json.Unmarshal([]byte(mcpText(res.Content)), &issue); err != nil {
		return nil, fmt.Errorf("parse issue response: %w", err)
	}

	if issue.Number == 0 || issue.Title == "" {
		return nil, fmt.Errorf("%s did not return a valid issue for %s/%s#%d (does it exist?)", tool, owner, repo, num)
	}

	if issue.PullRequest != nil {
		return nil, fmt.Errorf("%s/%s#%d is a pull request, not an issue", owner, repo, num)
	}

	return &issue, nil
}

func postComment(ctx context.Context, client *mcp.Client, tool, owner, repo string, num int, body string) error {
	res, err := client.CallTool(ctx, tool, map[string]any{
		"owner": owner, "repo": repo, "issue_number": num, "body": body,
	})
	if err != nil {
		return fmt.Errorf("%s via MCP: %w", tool, err)
	}
	if res.IsError {
		return fmt.Errorf("%s via MCP: %s", tool, mcp.FormatContent(res.Content, true))
	}
	return nil
}

func mcpText(content []mcp.ContentBlock) string {
	var parts []string
	for _, block := range content {
		if tc, ok := mcp.ParseTextContent(block); ok {
			parts = append(parts, tc.Text)
		}
	}
	return strings.Join(parts, "\n")
}

func envOr(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func llmConfig() sdk.Config {
	provider := envOr("LLM_PROVIDER", "openai")
	keyVars := map[string]string{
		"openai":     "OPENAI_API_KEY",
		"anthropic":  "ANTHROPIC_API_KEY",
		"openrouter": "OPENROUTER_API_KEY",
	}
	return sdk.Config{
		Provider: provider,
		Model:    envOr("LLM_MODEL", defaultModel(provider)),
		APIKey:   os.Getenv(keyVars[provider]),
	}
}

func defaultModel(provider string) string {
	switch provider {
	case "anthropic":
		return "claude-sonnet-4-20250514"
	case "openrouter":
		return "openai/gpt-4o"
	default:
		return "gpt-4o"
	}
}

func loadEnv(path string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil
		}
		return err
	}
	for lineNo, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		line = strings.TrimPrefix(line, "export ")
		key, val, ok := strings.Cut(line, "=")
		if !ok {
			return fmt.Errorf("line %d: expected KEY=VALUE, got %q", lineNo+1, line)
		}
		key = strings.TrimSpace(key)
		if key == "" {
			return fmt.Errorf("line %d: empty key", lineNo+1)
		}
		if os.Getenv(key) != "" {
			continue // already set — real environment wins
		}
		val = strings.TrimSpace(val)
		if n := len(val); n >= 2 && (val[0] == '"' && val[n-1] == '"' || val[0] == '\'' && val[n-1] == '\'') {
			val = val[1 : n-1]
		}
		os.Setenv(key, val)
	}
	return nil
}

func renderPrompt(issue *Issue) (string, error) {
	tpl, err := os.ReadFile(PROMPT_PATH)
	if err != nil {
		return "", fmt.Errorf("read prompt template: %w", err)
	}
	return strings.NewReplacer(
		"{{title}}", issue.Title,
		"{{body}}", issue.Body,
		"{{metadata}}", issueMetadata(issue),
	).Replace(string(tpl)), nil
}

func issueMetadata(issue *Issue) string {
	labels := make([]string, 0, len(issue.Labels))
	for _, l := range issue.Labels {
		labels = append(labels, l.Name)
	}
	meta := []string{
		"- Number: " + strconv.Itoa(issue.Number),
		"- State: " + issue.State,
		"- Reporter: @" + issue.User.Login,
		"- Labels: " + strings.Join(labels, ", "),
		"- URL: " + issue.HTMLURL,
		"- Created: " + issue.CreatedAt,
		"- Updated: " + issue.UpdatedAt,
	}
	if issue.Milestone != nil {
		meta = append(meta, "- Milestone: "+issue.Milestone.Title)
	}
	return strings.Join(meta, "\n")
}
