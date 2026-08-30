package sdk

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/zendev-sh/goai/mcp"
)

type MCPConfig struct {
	Name string
	Version string
	Command string
	Args []string
	Env map[string]string
	URL string
	Headers map[string]string
	RequestTimeout time.Duration
}

type MCPClient struct {
	client *mcp.Client
}

type MCPTool struct {
	Name string
}

func NewMCPClient(ctx context.Context, cfg MCPConfig) (*MCPClient, error) {
	if cfg.Name == "" || (cfg.Command == "" && cfg.URL == "") {
		return nil, fmt.Errorf("sdk: mcp config requires a name and either a command or a URL")
	}
	if cfg.Version == "" {
		cfg.Version = "0.0.0"
	}

	var transport mcp.Transport
	if cfg.URL != "" {
		opts := []mcp.HTTPTransportOption{}
		if len(cfg.Headers) > 0 {
			opts = append(opts, mcp.WithHTTPHeaders(cfg.Headers))
		}
		transport = mcp.NewHTTPTransport(cfg.URL, opts...)
	} else {
		transport = mcp.NewStdioTransport(cfg.Command, cfg.Args, mcp.WithStdioEnv(cfg.Env))
	}

	opts := []mcp.ClientOption{mcp.WithTransport(transport)}
	if cfg.RequestTimeout > 0 {
		opts = append(opts, mcp.WithRequestTimeout(cfg.RequestTimeout))
	}

	client := mcp.NewClient(cfg.Name, cfg.Version, opts...)
	if err := client.Connect(ctx); err != nil {
		return nil, fmt.Errorf("sdk: connect to MCP server %q: %w", cfg.Command, err)
	}
	return &MCPClient{client: client}, nil
}

func (c *MCPClient) Close() error {
	if c == nil || c.client == nil {
		return nil
	}
	return c.client.Close()
}

func (c *MCPClient) ListTools(ctx context.Context) ([]MCPTool, error) {
	res, err := c.client.ListTools(ctx, nil)
	if err != nil {
		return nil, fmt.Errorf("sdk: list tools: %w", err)
	}
	tools := make([]MCPTool, 0, len(res.Tools))
	for _, t := range res.Tools {
		tools = append(tools, MCPTool{Name: t.Name})
	}
	return tools, nil
}

func (c *MCPClient) Tools(ctx context.Context) ([]Tool, error) {
	res, err := c.client.ListTools(ctx, nil)
	if err != nil {
		return nil, fmt.Errorf("sdk: list tools: %w", err)
	}
	converted := mcp.ConvertTools(c.client, res.Tools)
	out := make([]Tool, 0, len(converted))
	for _, t := range converted {
		out = append(out, Tool{
			Name: t.Name,
			Description: t.Description,
			InputSchema: t.InputSchema,
			Execute: t.Execute,
		})
	}
	return out, nil
}

func (c *MCPClient) CallTool(ctx context.Context, name string, args map[string]any) (string, error) {
	res, err := c.client.CallTool(ctx, name, args)
	if err != nil {
		return "", fmt.Errorf("mcp tool %q: %w", name, err)
	}
	if res.IsError {
		return "", fmt.Errorf("mcp tool %q: %s", name, mcp.FormatContent(res.Content, true))
	}
	return mcpText(res.Content), nil
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
