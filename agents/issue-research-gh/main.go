package main

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/nikelaz/droid-bay/helpers"
	"github.com/nikelaz/droid-bay/sdk"
)

const SYSTEM_PROMPT_PATH = "./system-prompt.md"
const USER_PROMPT_PATH = "./user-prompt.md"
const MODEL_DEFAULTS_PATH = "./model-defaults.json"
const TEMPERATURE = 0.2
const MAX_STEPS = 300
const RUN_TIMEOUT = 90 * time.Minute
const CODEBASES_ROOT = ".codebases"
const LOGS_DIR = ".logs"

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
	Number int `json:"number"`
	Title string `json:"title"`
	Body string `json:"body"`
	State string `json:"state"`
	HTMLURL string `json:"html_url"`
	CreatedAt string `json:"created_at"`
	UpdatedAt string `json:"updated_at"`
	User User `json:"user"`
	Labels []Labels `json:"labels"`
	Milestone *Milestone `json:"milestone"`
	PullRequest *struct{} `json:"pull_request"`
}

type options struct {
	owner string
	repo string
	issue int
	envFile string
	model string
	codebase string
}

func parseArgs() options {
	owner := flag.String("owner", "", "repository owner (user or org)")
	repo := flag.String("repo", "", "repository name")
	num := flag.Int("issue", 0, "issue number")
	envFile := flag.String("env", ".env", "path to an environment file (optional)")
	model := flag.String("model", "", "model to use; overrides model-defaults.json and skips its reasoning effort")
	codebase := flag.String("codebase", "", "path to a local repo checkout (default: auto-clones to .codebases/<owner>/<repo>)")
	flag.Parse()

	return options{
		owner: *owner,
		repo: *repo,
		issue: *num,
		envFile: *envFile,
		model: *model,
		codebase: *codebase,
	}
}

func main() {
	opts := parseArgs()

	log.SetFlags(0)

	closeLog, err := helpers.SetupRunLog(LOGS_DIR)
	if err != nil {
		log.Printf("continuing without a run log: %v", err)
	} else {
		defer closeLog()
	}

	if err := helpers.LoadEnv(opts.envFile); err != nil {
		log.Fatalf("load env file: %v", err)
	}

	if opts.owner == "" || opts.repo == "" || opts.issue <= 0 {
		log.Fatal("usage: issue-research-gh -owner <owner> -repo <repo> -issue <number> [-env <file>]")
	}

	if os.Getenv("GITHUB_TOKEN") == "" {
		log.Fatal("GITHUB_TOKEN is required (set it in the env file)")
	}

	ctx, cancel := context.WithTimeout(context.Background(), RUN_TIMEOUT)
	defer cancel()

	ghClient := connectGitHubMCP(ctx, os.Getenv("GITHUB_TOKEN"))
	defer ghClient.Close()

	if opts.codebase == "" {
		clonedPath, cloneErr := cloneCodebase(opts.owner, opts.repo)
		if cloneErr != nil {
			log.Fatal(cloneErr)
		}
		opts.codebase = clonedPath
		log.Printf("cloned %s/%s to %s", opts.owner, opts.repo, opts.codebase)
	}

	codeClient := connectCodebaseMCP(ctx, opts.codebase)
	defer codeClient.Close()

	getIssueTool, commentTool, err := resolveTools(ctx, ghClient)
	if err != nil {
		log.Fatal(err)
	}

	issue, err := getIssue(ctx, ghClient, getIssueTool, opts.owner, opts.repo, opts.issue)
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("issue #%d: %s (state=%s)", issue.Number, issue.Title, issue.State)

	prompt, err := renderPrompt(pathFor(USER_PROMPT_PATH), issue)
	if err != nil {
		log.Fatal(err)
	}

	systemPrompt, err := os.ReadFile(pathFor(SYSTEM_PROMPT_PATH))
	if err != nil {
		log.Fatalf("read system prompt: %v", err)
	}

	cfg, err := sdk.ConfigFromDefaults(pathFor(MODEL_DEFAULTS_PATH))
	if err != nil {
		if opts.model == "" || !errors.Is(err, os.ErrNotExist) {
			log.Fatal(err)
		}
		cfg = sdk.ConfigFromEnv()
	}
	if opts.model != "" {
		cfg.Model = opts.model
		cfg.Effort = ""
	}

	webClient := connectWebSearchMCP(ctx)
	defer webClient.Close()

	webTools, err := webClient.Tools(ctx)
	if err != nil {
		log.Fatalf("list web search tools: %v", err)
	}
	tools := make([]sdk.Tool, 0, len(webTools)+4)
	for _, t := range webTools {
		tools = append(tools, prepareWebTool(t, cfg.Model))
	}

	codeTools, err := codeClient.Tools(ctx)
	if err != nil {
		log.Fatal(err)
	}
	tools = append(tools, codeTools...)

	log.Printf("generating research report (max %d steps)", MAX_STEPS)
	report, err := sdk.Generate(ctx, cfg, string(systemPrompt), prompt,
		sdk.WithTemperature(TEMPERATURE),
		sdk.WithTools(tools...),
		sdk.WithMaxSteps(MAX_STEPS),
		sdk.WithDebugLog(log.Default()),
	)
	if err != nil {
		log.Fatal(err)
	}

	if strings.TrimSpace(report) == "" {
		log.Fatal("LLM returned an empty report; nothing to post")
	}

	log.Printf("\n--- research report (%d bytes) ---\n%s", len(report), report)

	if err := postComment(ctx, ghClient, commentTool, opts.owner, opts.repo, opts.issue, report); err != nil {
		log.Fatal(err)
	}

	log.Printf("\nresearch posted to %s/%s#%d", opts.owner, opts.repo, opts.issue)
}

func connectGitHubMCP(ctx context.Context, token string) *sdk.MCPClient {
	argv, err := helpers.SplitArgs(helpers.EnvOr("GITHUB_MCP_CMD", "npx -y @modelcontextprotocol/server-github"))
	if err != nil {
		log.Fatalf("parse GITHUB_MCP_CMD: %v", err)
	}

	if len(argv) == 0 {
		log.Fatal("GITHUB_MCP_CMD is empty; expected at least a command")
	}

	client, err := sdk.NewMCPClient(ctx, sdk.MCPConfig{
		Name: "issue-research-gh",
		Version: "1.0.0",
		Command: argv[0],
		Args: argv[1:],
		Env: map[string]string{
			"GITHUB_TOKEN": token,
			"GITHUB_PERSONAL_ACCESS_TOKEN": token,
		},
		RequestTimeout: 2 * time.Minute,
	})
	if err != nil {
		log.Fatalf("connect to GitHub MCP server: %v (is it installed / Node available?)", err)
	}

	return client
}

func cloneCodebase(owner, repo string) (string, error) {
	dir := filepath.Join(CODEBASES_ROOT, owner, repo)
	if err := os.RemoveAll(dir); err != nil {
		return "", fmt.Errorf("remove previous checkout: %w", err)
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return "", fmt.Errorf("create codebase dir: %w", err)
	}
	cmd := exec.Command("git", "clone", "--depth", "1",
		"git@github.com:"+owner+"/"+repo+".git", ".")
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), "GIT_TERMINAL_PROMPT=0")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("git clone %s/%s: %v: %s", owner, repo, err, strings.TrimSpace(string(out)))
	}
	return dir, nil
}

func connectCodebaseMCP(ctx context.Context, root string) *sdk.MCPClient {
	var argv []string
	if cmd := os.Getenv("CODEBASE_MCP_CMD"); cmd != "" {
		var err error
		argv, err = helpers.SplitArgs(cmd)
		if err != nil {
			log.Fatalf("parse CODEBASE_MCP_CMD: %v", err)
		}
	} else {
		argv = []string{"npx", "-y", "@modelcontextprotocol/server-filesystem", root}
	}

	if len(argv) == 0 {
		log.Fatal("CODEBASE_MCP_CMD is empty; expected at least a command")
	}

	client, err := sdk.NewMCPClient(ctx, sdk.MCPConfig{
		Name: "issue-research-gh-codebase",
		Version: "1.0.0",
		Command: argv[0],
		Args: argv[1:],
		RequestTimeout: 2 * time.Minute,
	})
	if err != nil {
		log.Fatalf("connect to codebase MCP server: %v (is it installed / Node available?)", err)
	}

	return client
}

func pathFor(name string) string {
	if filepath.IsAbs(name) {
		return name
	}
	if exe, err := os.Executable(); err == nil {
		p := filepath.Join(filepath.Dir(exe), name)
		if _, err := os.Stat(p); err == nil {
			return p
		}
	}
	return name
}

func connectWebSearchMCP(ctx context.Context) *sdk.MCPClient {
	if cmd := os.Getenv("WEB_SEARCH_MCP_CMD"); cmd != "" {
		argv, err := helpers.SplitArgs(cmd)
		if err != nil {
			log.Fatalf("parse WEB_SEARCH_MCP_CMD: %v", err)
		}
		client, err := sdk.NewMCPClient(ctx, sdk.MCPConfig{
			Name: "issue-research-gh-web",
			Command: argv[0],
			Args: argv[1:],
			RequestTimeout: 2 * time.Minute,
		})
		if err != nil {
			log.Fatalf("connect to web search MCP server: %v", err)
		}
		return client
	}

	client, err := sdk.NewMCPClient(ctx, sdk.MCPConfig{
		Name: "issue-research-gh-web",
		URL: helpers.EnvOr("WEB_SEARCH_URL", "https://search.parallel.ai/mcp"),
		RequestTimeout: 2 * time.Minute,
	})
	if err != nil {
		log.Fatalf("connect to web search MCP server: %v", err)
	}
	return client
}

var searchSessionID = newSearchSessionID()

func newSearchSessionID() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		return strconv.FormatInt(time.Now().UnixNano(), 16)
	}
	b[6] = (b[6] & 0x0f) | 0x40
	b[8] = (b[8] & 0x3f) | 0x80
	return fmt.Sprintf("%x-%x-%x-%x-%x", b[0:4], b[4:6], b[6:8], b[8:10], b[10:16])
}

func prepareWebTool(t sdk.Tool, model string) sdk.Tool {
	t.InputSchema = hideSchemaProps(t.InputSchema, "session_id", "model_name")
	execute := t.Execute
	t.Execute = func(ctx context.Context, input json.RawMessage) (string, error) {
		var args map[string]any
		if len(input) > 0 {
			if err := json.Unmarshal(input, &args); err != nil {
				return "", err
			}
		}
		if args == nil {
			args = make(map[string]any)
		}
		args["session_id"] = searchSessionID
		if model != "" {
			args["model_name"] = model
		}
		in, err := json.Marshal(args)
		if err != nil {
			return "", err
		}
		return execute(ctx, in)
	}
	return t
}

func hideSchemaProps(schema json.RawMessage, props ...string) json.RawMessage {
	var s map[string]any
	if err := json.Unmarshal(schema, &s); err != nil {
		return schema
	}
	properties, ok := s["properties"].(map[string]any)
	if !ok {
		return schema
	}
	for _, p := range props {
		delete(properties, p)
	}
	out, err := json.Marshal(s)
	if err != nil {
		return schema
	}
	return out
}

func resolveTools(ctx context.Context, client *sdk.MCPClient) (getIssueTool, commentTool string, err error) {
	tools, err := client.ListTools(ctx)
	if err != nil {
		return "", "", err
	}

	for _, t := range tools {
		switch t.Name {
		case "get_issue":
			getIssueTool = t.Name
		case "add_issue_comment":
			commentTool = t.Name
		}
	}

	if getIssueTool == "" {
		return "", "", fmt.Errorf("no get_issue tool found (is the GitHub MCP server running?)")
	}

	if commentTool == "" {
		return "", "", fmt.Errorf("no add_issue_comment tool found")
	}

	return getIssueTool, commentTool, nil
}

func getIssue(ctx context.Context, client *sdk.MCPClient, tool, owner, repo string, num int) (*Issue, error) {
	text, err := client.CallTool(ctx, tool, map[string]any{
		"owner": owner, "repo": repo, "issue_number": num,
	})
	if err != nil {
		return nil, err
	}

	var issue Issue
	if err := json.Unmarshal([]byte(text), &issue); err != nil {
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

func postComment(ctx context.Context, client *sdk.MCPClient, tool, owner, repo string, num int, body string) error {
	_, err := client.CallTool(ctx, tool, map[string]any{
		"owner": owner, "repo": repo, "issue_number": num, "body": body,
	})
	return err
}

func renderPrompt(path string, issue *Issue) (string, error) {
	tpl, err := os.ReadFile(path)
	if err != nil {
		return "", fmt.Errorf("read user prompt template: %w", err)
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
