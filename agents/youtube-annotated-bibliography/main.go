package main

import (
	"bytes"
	"context"
	_ "embed"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"regexp"
	"strings"
	"time"

	"github.com/nikelaz/droid-bay/helpers"
	"github.com/nikelaz/droid-bay/sdk"
)

const (
	TEMPERATURE      = 0.2
	MAX_STEPS        = 300
	RUN_TIMEOUT      = 90 * time.Minute
	LOGS_DIR         = ".logs"
	DOCUMENT_TITLE   = "LLM Generated Annotated Bibliography"
	LINEAR_GRAPHQL_URL = "https://api.linear.app/graphql"
)

//go:embed system-prompt.md
var systemPrompt string

//go:embed user-prompt.md
var userPrompt string

//go:embed model-defaults.json
var modelDefaults []byte

type LinearIssue struct {
	ID          string `json:"id"`
	Identifier  string `json:"identifier"`
	Title       string `json:"title"`
	Description string `json:"description"`
	URL         string `json:"url"`
}

type options struct {
	issue string
	model string
}

func parseArgs() options {
	issue := flag.String("issue", "", "linear issue identifier (ENG-123), UUID, or URL")
	model := flag.String("model", "", "model to use; overrides model-defaults.json and skips its reasoning effort")
	flag.Parse()

	return options{
		issue: *issue,
		model: *model,
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

	if opts.issue == "" {
		log.Fatal("usage: youtube-annotated-bibliography -issue <identifier|uuid|url>")
	}

	missing := sdk.MissingLLMEnv()
	if os.Getenv("LINEAR_API_KEY") == "" && os.Getenv("LINEAR_MCP_URL") == "" {
		missing = append(missing, helpers.MissingEnv("LINEAR_API_KEY")...)
	}
	if len(missing) > 0 {
		log.Fatalf("missing environment variables: %s (set them in your shell environment)", strings.Join(missing, ", "))
	}

	ctx, cancel := context.WithTimeout(context.Background(), RUN_TIMEOUT)
	defer cancel()

	cfg, err := sdk.ConfigForRun(modelDefaults, opts.model)
	if err != nil {
		log.Fatal(err)
	}

	var (
		linearClient *sdk.MCPClient
		linearTools  []sdk.MCPTool
		commentTool  string
		issue        *LinearIssue
	)

	if apiKey := os.Getenv("LINEAR_API_KEY"); apiKey != "" && os.Getenv("LINEAR_MCP_URL") == "" {
		issue, err = fetchIssueViaAPI(ctx, apiKey, opts.issue)
		if err != nil {
			log.Fatal(err)
		}
	} else {
		linearClient = connectLinearMCP(ctx)
		defer linearClient.Close()

		var toolsErr error
		linearTools, toolsErr = linearClient.ListTools(ctx)
		if toolsErr != nil {
			log.Fatal(toolsErr)
		}

		getIssueTool := findOptionalTool(linearTools, "get_issue", "linear_get_issue", "getIssue")
		searchIssueTool := findOptionalTool(linearTools, "linear_search_issues", "search_issues", "searchIssues")
		if getIssueTool == "" && searchIssueTool == "" {
			log.Fatalf("linear mcp: none of the tools get_issue, linear_get_issue, getIssue, linear_search_issues were found (tools available: %s)", toolNames(linearTools))
		}
		if getIssueTool == "" {
			log.Printf("no dedicated issue getter on linear mcp; fetching %s via %s", opts.issue, searchIssueTool)
		}

		var commentErr error
		commentTool, commentErr = findTool(linearTools, "create_comment", "linear_create_comment", "createComment", "linear_add_comment", "add_comment", "addComment")
		if commentErr != nil {
			log.Fatalf("linear mcp: %v (tools available: %s)", commentErr, toolNames(linearTools))
		}

		issue, err = fetchIssue(ctx, linearClient, getIssueTool, searchIssueTool, opts.issue)
		if err != nil {
			log.Fatal(err)
		}
	}

	log.Printf("issue %s: %s", issue.Identifier, issue.Title)

	prompt := renderPrompt(userPrompt, issue)

	webTools, stopSearch, err := sdk.WebSearchTools(ctx)
	if err != nil {
		log.Fatal(err)
	}
	defer stopSearch()

	log.Printf("generating annotated bibliography (max %d steps)", MAX_STEPS)
	bibliography, err := sdk.Generate(ctx, cfg, systemPrompt, prompt,
		sdk.WithTemperature(TEMPERATURE),
		sdk.WithTools(webTools...),
		sdk.WithMaxSteps(MAX_STEPS),
		sdk.WithDebugLog(log.Default()),
	)
	if err != nil {
		log.Fatal(err)
	}

	if strings.TrimSpace(bibliography) == "" {
		log.Fatal("LLM returned an empty bibliography; nothing to attach")
	}

	log.Printf("\n--- annotated bibliography (%d bytes) ---\n%s", len(bibliography), bibliography)

	if linearClient == nil {
		body := fmt.Sprintf("## %s\n\n%s", DOCUMENT_TITLE, bibliography)
		if err := postCommentViaAPI(ctx, os.Getenv("LINEAR_API_KEY"), issue, body); err != nil {
			log.Fatal(err)
		}
		log.Printf("bibliography commented on %s", issue.Identifier)
		return
	}

	var docURL string
	if createDocTool := findOptionalTool(linearTools, "create_document", "linear_create_document", "createDocument"); createDocTool != "" {
		docText, docErr := linearClient.CallTool(ctx, createDocTool, map[string]any{
			"title": DOCUMENT_TITLE, "content": bibliography,
		})
		if docErr != nil {
			log.Printf("create document failed, falling back to comment: %v", docErr)
		} else {
			docURL = extractDocLink(docText, issue.URL)
			if docURL != "" {
				log.Printf("document created: %s", docURL)
			} else {
				log.Printf("document created but no link found in response: %.200s", docText)
			}
		}
	}

	if docURL != "" {
		if attachTool := findOptionalTool(linearTools, "create_attachment", "linear_create_attachment", "attachment_create", "createAttachment"); attachTool != "" && issue.ID != "" {
			_, err := linearClient.CallTool(ctx, attachTool, map[string]any{
				"issueId": issue.ID, "title": DOCUMENT_TITLE, "url": docURL,
			})
			if err == nil {
				log.Printf("document attached to %s", issue.Identifier)
				return
			}
			log.Printf("attachment failed, falling back to comment: %v", err)
		}
		linkBody := fmt.Sprintf("%s\n\n%s", DOCUMENT_TITLE, docURL)
		if err := postComment(ctx, linearClient, commentTool, issue, opts.issue, linkBody); err != nil {
			log.Fatal(err)
		}
		log.Printf("document link commented on %s", issue.Identifier)
		return
	}

	body := fmt.Sprintf("## %s\n\n%s", DOCUMENT_TITLE, bibliography)
	if err := postComment(ctx, linearClient, commentTool, issue, opts.issue, body); err != nil {
		log.Fatal(err)
	}
	log.Printf("bibliography commented on %s", issue.Identifier)
}

func connectLinearMCP(ctx context.Context) *sdk.MCPClient {
	url := os.Getenv("LINEAR_MCP_URL")
	if url == "" {
		log.Fatal("linear mcp: set LINEAR_MCP_URL (direct API access needs LINEAR_API_KEY)")
	}
	client, err := sdk.ConnectHTTP(ctx, "youtube-annotated-bibliography-linear", url)
	if err != nil {
		log.Fatalf("connect to linear MCP server at %s: %v", url, err)
	}
	return client
}

func findTool(tools []sdk.MCPTool, names ...string) (string, error) {
	if name := findOptionalTool(tools, names...); name != "" {
		return name, nil
	}
	return "", fmt.Errorf("none of the tools %s were found", strings.Join(names, ", "))
}

func findOptionalTool(tools []sdk.MCPTool, names ...string) string {
	for _, t := range tools {
		for _, name := range names {
			if t.Name == name {
				return t.Name
			}
		}
	}
	return ""
}

func toolNames(tools []sdk.MCPTool) string {
	names := make([]string, 0, len(tools))
	for _, t := range tools {
		names = append(names, t.Name)
	}
	return strings.Join(names, ", ")
}

func fetchIssue(ctx context.Context, client *sdk.MCPClient, getTool, searchTool, ref string) (*LinearIssue, error) {
	if getTool != "" {
		for _, key := range []string{"issue", "issueId", "id"} {
			text, err := client.CallTool(ctx, getTool, map[string]any{key: ref})
			if err != nil {
				continue
			}
			if issue := parseIssue(text); issue != nil {
				return fillIssue(issue, ref), nil
			}
		}
	}
	if searchTool != "" {
		text, err := client.CallTool(ctx, searchTool, map[string]any{"query": ref})
		if err != nil {
			log.Printf("linear mcp: %s failed: %v", searchTool, err)
		} else if issue := matchIssue(parseIssues(text), ref); issue != nil {
			return fillIssue(issue, ref), nil
		}
	}
	return nil, fmt.Errorf("linear mcp: no tool returned a valid issue for %s (does it exist?)", ref)
}

func fetchIssueViaAPI(ctx context.Context, apiKey, ref string) (*LinearIssue, error) {
	ref = normalizeIssueRef(ref)
	data, err := linearGraphQL(ctx, apiKey,
		`query($id: String!) { issue(id: $id) { id identifier title description url } }`,
		map[string]any{"id": ref},
	)
	if err != nil {
		return nil, fmt.Errorf("linear api: fetch issue %s: %w", ref, err)
	}
	var parsed struct {
		Data struct {
			Issue *LinearIssue `json:"issue"`
		} `json:"data"`
	}
	if err := json.Unmarshal(data, &parsed); err != nil {
		return nil, fmt.Errorf("linear api: decode issue %s: %w", ref, err)
	}
	if parsed.Data.Issue == nil || parsed.Data.Issue.Title == "" {
		return nil, fmt.Errorf("linear api: no issue found for %s", ref)
	}
	return parsed.Data.Issue, nil
}

func postCommentViaAPI(ctx context.Context, apiKey string, issue *LinearIssue, body string) error {
	_, err := linearGraphQL(ctx, apiKey,
		`mutation($input: CommentCreateInput!) { commentCreate(input: $input) { success } }`,
		map[string]any{"input": map[string]any{"issueId": issue.ID, "body": body}},
	)
	if err != nil {
		return fmt.Errorf("linear api: comment on %s: %w", issue.Identifier, err)
	}
	return nil
}

func linearGraphQL(ctx context.Context, apiKey, query string, variables map[string]any) ([]byte, error) {
	payload, err := json.Marshal(map[string]any{"query": query, "variables": variables})
	if err != nil {
		return nil, err
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, LINEAR_GRAPHQL_URL, bytes.NewReader(payload))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", apiKey)
	req.Header.Set("Content-Type", "application/json")
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, err
	}
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("status %d: %.200s", resp.StatusCode, data)
	}
	var errs struct {
		Errors []struct {
			Message string `json:"message"`
		} `json:"errors"`
	}
	if json.Unmarshal(data, &errs) == nil && len(errs.Errors) > 0 {
		msgs := make([]string, 0, len(errs.Errors))
		for _, e := range errs.Errors {
			msgs = append(msgs, e.Message)
		}
		return nil, fmt.Errorf("%s", strings.Join(msgs, "; "))
	}
	return data, nil
}

func normalizeIssueRef(ref string) string {
	if !strings.HasPrefix(ref, "http") {
		return ref
	}
	if match := uuidRe.FindString(ref); match != "" {
		return match
	}
	if match := issueIdentifierRe.FindString(ref); match != "" {
		return match
	}
	return ref
}

func parseIssues(text string) []*LinearIssue {
	var list []*LinearIssue
	if err := json.Unmarshal([]byte(text), &list); err == nil {
		return issuesWithTitles(list)
	}

	var wrapped struct {
		Issues []*LinearIssue `json:"issues"`
		Nodes  []*LinearIssue `json:"nodes"`
	}
	if err := json.Unmarshal([]byte(text), &wrapped); err == nil {
		if len(wrapped.Issues) > 0 {
			return issuesWithTitles(wrapped.Issues)
		}
		return issuesWithTitles(wrapped.Nodes)
	}
	return nil
}

func issuesWithTitles(list []*LinearIssue) []*LinearIssue {
	kept := make([]*LinearIssue, 0, len(list))
	for _, issue := range list {
		if issue != nil && issue.Title != "" {
			kept = append(kept, issue)
		}
	}
	return kept
}

func matchIssue(candidates []*LinearIssue, ref string) *LinearIssue {
	refs := issueRefs(ref)
	for _, issue := range candidates {
		for _, field := range []string{issue.Identifier, issue.ID, issue.URL} {
			field = strings.ToLower(strings.TrimSpace(field))
			for _, want := range refs {
				if field != "" && field == want {
					return issue
				}
			}
		}
	}
	if len(candidates) == 1 {
		return candidates[0]
	}
	return nil
}

func issueRefs(ref string) []string {
	refs := []string{strings.ToLower(strings.TrimSpace(ref))}
	if match := issueIdentifierRe.FindString(ref); match != "" {
		refs = append(refs, strings.ToLower(match))
	}
	if match := uuidRe.FindString(ref); match != "" {
		refs = append(refs, strings.ToLower(match))
	}
	return refs
}

func fillIssue(issue *LinearIssue, ref string) *LinearIssue {
	if issue.Identifier == "" {
		if match := issueIdentifierRe.FindString(ref); match != "" {
			issue.Identifier = match
		} else {
			issue.Identifier = ref
		}
	}
	if issue.URL == "" && strings.HasPrefix(ref, "http") {
		issue.URL = ref
	}
	return issue
}

func postComment(ctx context.Context, client *sdk.MCPClient, tool string, issue *LinearIssue, ref, body string) error {
	var lastErr error
	for _, key := range []string{"issueId", "issue"} {
		value := ref
		if issue.ID != "" {
			value = issue.ID
		}
		_, err := client.CallTool(ctx, tool, map[string]any{key: value, "body": body})
		if err == nil {
			return nil
		}
		lastErr = err
	}
	return lastErr
}

func parseIssue(text string) *LinearIssue {
	var direct LinearIssue
	if err := json.Unmarshal([]byte(text), &direct); err == nil && direct.Title != "" {
		return &direct
	}

	var wrapped struct {
		Issue *LinearIssue `json:"issue"`
	}
	if err := json.Unmarshal([]byte(text), &wrapped); err == nil && wrapped.Issue != nil && wrapped.Issue.Title != "" {
		return wrapped.Issue
	}

	return nil
}

func renderPrompt(tpl string, issue *LinearIssue) string {
	meta := []string{
		"- Identifier: " + issue.Identifier,
		"- URL: " + issue.URL,
	}
	return strings.NewReplacer(
		"{{title}}", issue.Title,
		"{{description}}", issue.Description,
		"{{metadata}}", strings.Join(meta, "\n"),
	).Replace(tpl)
}

var (
	docLinkRe         = regexp.MustCompile(`https?://[^\s)\]"'<>]+/document/[^\s)\]"'<>]*`)
	uuidRe            = regexp.MustCompile(`[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}`)
	orgRe             = regexp.MustCompile(`https://linear\.app/([^/\s)]+)`)
	issueIdentifierRe = regexp.MustCompile(`(?i)\b[A-Z0-9]{2,10}-[0-9]+\b`)
)

func extractDocLink(text, issueURL string) string {
	if match := docLinkRe.FindString(text); match != "" {
		return match
	}

	id := uuidRe.FindString(text)
	if id == "" {
		return ""
	}

	match := orgRe.FindStringSubmatch(issueURL)
	if match == nil {
		return ""
	}

	return fmt.Sprintf("https://linear.app/%s/document/%s", match[1], id)
}
