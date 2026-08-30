package sdk

import (
	"context"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"time"

	"github.com/nikelaz/droid-bay/helpers"
)

const defaultWebSearchURL = "https://search.parallel.ai/mcp"

var webSearchSession = newSessionID()

func WebSearchTools(ctx context.Context, name string, cfg Config) (*MCPClient, []Tool, error) {
	var client *MCPClient
	var err error
	if cmd := os.Getenv("WEB_SEARCH_MCP_CMD"); cmd != "" {
		argv, splitErr := helpers.SplitArgs(cmd)
		if splitErr != nil {
			return nil, nil, fmt.Errorf("sdk: parse WEB_SEARCH_MCP_CMD: %w", splitErr)
		}
		if len(argv) == 0 {
			return nil, nil, fmt.Errorf("sdk: WEB_SEARCH_MCP_CMD is empty; expected at least a command")
		}
		client, err = ConnectStdio(ctx, name, argv[0], argv[1:], nil)
	} else {
		client, err = ConnectHTTP(ctx, name, helpers.EnvOr("WEB_SEARCH_URL", defaultWebSearchURL))
	}
	if err != nil {
		return nil, nil, fmt.Errorf("sdk: connect to web search MCP server: %w", err)
	}

	tools, err := client.Tools(ctx)
	if err != nil {
		return nil, nil, fmt.Errorf("sdk: list web search tools: %w", err)
	}

	wrapped := make([]Tool, 0, len(tools))
	for _, t := range tools {
		wrapped = append(wrapped, wrapWebTool(t, cfg.Model))
	}
	return client, wrapped, nil
}

func wrapWebTool(t Tool, model string) Tool {
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
		args["session_id"] = webSearchSession
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

func newSessionID() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		return strconv.FormatInt(time.Now().UnixNano(), 16)
	}
	b[6] = (b[6] & 0x0f) | 0x40
	b[8] = (b[8] & 0x3f) | 0x80
	return fmt.Sprintf("%x-%x-%x-%x-%x", b[0:4], b[4:6], b[6:8], b[8:10], b[10:16])
}