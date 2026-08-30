package sdk

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"
)

const (
	defaultSearxngURL = "http://127.0.0.1:8888"
	searxngReadyTimeout = 5 * time.Second
	searchHTTPTimeout = 30 * time.Second
	maxSearchResults = 15
)

func WebSearchTools(ctx context.Context) ([]Tool, func(), error) {
	baseURL := defaultSearxngURL
	if raw := os.Getenv("SEARXNG_URL"); raw != "" {
		baseURL = strings.TrimRight(raw, "/")
	}
	inst := &searxngInstance{baseURL: baseURL}
	if err := inst.waitReady(ctx, searxngReadyTimeout); err != nil {
		return nil, nil, fmt.Errorf("sdk: no SearXNG instance at %s: %w; install SearXNG (https://docs.searxng.org) and run it locally, or point SEARXNG_URL at a running instance", inst.baseURL, err)
	}
	return []Tool{searxngTool(inst)}, func() {}, nil
}

type searxngInstance struct {
	baseURL string
}

func (s *searxngInstance) waitReady(ctx context.Context, timeout time.Duration) error {
	client := &http.Client{Timeout: 2 * time.Second}
	deadline := time.Now().Add(timeout)
	for {
		req, err := http.NewRequestWithContext(ctx, http.MethodGet, s.baseURL+"/search?q=ping&format=json", nil)
		if err != nil {
			return err
		}
		resp, err := client.Do(req)
		if err == nil {
			resp.Body.Close()
			if resp.StatusCode == http.StatusOK {
				return nil
			}
		}
		if time.Now().After(deadline) {
			return fmt.Errorf("not ready after %s", timeout)
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(250 * time.Millisecond):
		}
	}
}

func (s *searxngInstance) search(ctx context.Context, query string) (string, error) {
	values := url.Values{}
	values.Set("q", query)
	values.Set("format", "json")
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, s.baseURL+"/search?"+values.Encode(), nil)
	if err != nil {
		return "", err
	}
	client := &http.Client{Timeout: searchHTTPTimeout}
	resp, err := client.Do(req)
	if err != nil {
		return "", fmt.Errorf("web_search: %w", err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
	if err != nil {
		return "", fmt.Errorf("web_search: read response: %w", err)
	}
	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("web_search: searxng returned status %d: %.200s", resp.StatusCode, body)
	}
	var parsed struct {
		Results []struct {
			Title string `json:"title"`
			URL string `json:"url"`
			Content string `json:"content"`
		} `json:"results"`
	}
	if err := json.Unmarshal(body, &parsed); err != nil {
		return "", fmt.Errorf("web_search: parse searxng response: %w", err)
	}
	if len(parsed.Results) == 0 {
		return "No results found.", nil
	}
	var b strings.Builder
	for i, r := range parsed.Results {
		if i == maxSearchResults {
			break
		}
		fmt.Fprintf(&b, "%d. %s\n   %s\n", i+1, r.Title, r.URL)
		if snippet := strings.TrimSpace(r.Content); snippet != "" {
			fmt.Fprintf(&b, "   %s\n", snippet)
		}
		b.WriteString("\n")
	}
	return b.String(), nil
}

func searxngTool(inst *searxngInstance) Tool {
	return Tool{
		Name: "web_search",
		Description: "Search the web. Returns up to 15 results, each with a title, URL, and snippet.",
		InputSchema: json.RawMessage(`{
			"type": "object",
			"properties": {
				"query": {"type": "string", "description": "Search query"}
			},
			"required": ["query"]
		}`),
		Execute: func(ctx context.Context, input json.RawMessage) (string, error) {
			var args struct {
				Query string `json:"query"`
			}
			if err := json.Unmarshal(input, &args); err != nil {
				return "", err
			}
			if strings.TrimSpace(args.Query) == "" {
				return "", fmt.Errorf("web_search: query is required")
			}
			return inst.search(ctx, args.Query)
		},
	}
}
