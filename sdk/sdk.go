package sdk

import (
	"context"
	"fmt"

	"github.com/zendev-sh/goai"
	"github.com/zendev-sh/goai/provider"
	"github.com/zendev-sh/goai/provider/anthropic"
	"github.com/zendev-sh/goai/provider/openai"
	"github.com/zendev-sh/goai/provider/openrouter"
)

type Config struct {
	Provider string
	Model string
	APIKey string
}

func Model(cfg Config) (provider.LanguageModel, error) {
	if cfg.Provider == "" || cfg.Model == "" || cfg.APIKey == "" {
		return nil, fmt.Errorf("sdk: incomplete config: provider=%q model=%q apiKeySet=%t", cfg.Provider, cfg.Model, cfg.APIKey != "")
	}
	switch cfg.Provider {
	case "openai":
		return openai.Chat(cfg.Model, openai.WithAPIKey(cfg.APIKey)), nil
	case "anthropic":
		return anthropic.Chat(cfg.Model, anthropic.WithAPIKey(cfg.APIKey)), nil
	case "openrouter":
		return openrouter.Chat(cfg.Model, openrouter.WithAPIKey(cfg.APIKey)), nil
	default:
		return nil, fmt.Errorf("sdk: unknown provider %q (want openai, anthropic, or openrouter)", cfg.Provider)
	}
}

func Generate(ctx context.Context, cfg Config, system, prompt string, opts ...goai.Option) (string, error) {
	model, err := Model(cfg)
	if err != nil {
		return "", err
	}
	opts = append([]goai.Option{
		goai.WithSystem(system),
		goai.WithPrompt(prompt),
	}, opts...)
	result, err := goai.GenerateText(ctx, model, opts...)
	if err != nil {
		return "", fmt.Errorf("generate text: %w", err)
	}
	return result.Text, nil
}
