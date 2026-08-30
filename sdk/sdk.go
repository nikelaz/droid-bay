package sdk

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/nikelaz/droid-bay/helpers"
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
	Effort string
}

type Tool struct {
	Name string
	Description string
	InputSchema json.RawMessage
	Execute func(ctx context.Context, input json.RawMessage) (string, error)
}

type ModelDefault struct {
	Model string `json:"model"`
	Effort *string `json:"effort"`
}

type DebugLogger interface {
	Printf(format string, args ...any)
}

type Option func(*generateOptions)

type generateOptions struct {
	temperature float64
	tools []Tool
	maxSteps int
	debugLog DebugLogger
}

const defaultMaxSteps = 30

func WithTemperature(t float64) Option {
	return func(o *generateOptions) { o.temperature = t }
}

func WithTools(tools ...Tool) Option {
	return func(o *generateOptions) { o.tools = tools }
}

func WithMaxSteps(n int) Option {
	return func(o *generateOptions) { o.maxSteps = n }
}

func WithDebugLog(l DebugLogger) Option {
	return func(o *generateOptions) { o.debugLog = l }
}

func ConfigFromEnv() Config {
	provider := helpers.EnvOr("LLM_PROVIDER", "openai")
	keyVars := map[string]string{
		"openai": "OPENAI_API_KEY",
		"anthropic": "ANTHROPIC_API_KEY",
		"openrouter": "OPENROUTER_API_KEY",
	}
	return Config{
		Provider: provider,
		APIKey: os.Getenv(keyVars[provider]),
	}
}

func MissingLLMEnv() []string {
	provider := helpers.EnvOr("LLM_PROVIDER", "openai")
	keyVars := map[string]string{
		"openai": "OPENAI_API_KEY",
		"anthropic": "ANTHROPIC_API_KEY",
		"openrouter": "OPENROUTER_API_KEY",
	}
	keyVar, ok := keyVars[provider]
	if !ok {
		return []string{fmt.Sprintf("LLM_PROVIDER=%q is not one of openai, anthropic, openrouter", provider)}
	}
	return helpers.MissingEnv(keyVar)
}

func ConfigForRun(defaults []byte, model string) (Config, error) {
	cfg := ConfigFromEnv()
	if len(defaults) > 0 {
		var parsed map[string]ModelDefault
		if err := json.Unmarshal(defaults, &parsed); err != nil {
			return Config{}, fmt.Errorf("sdk: parse model defaults: %w", err)
		}
		if d, ok := parsed[cfg.Provider]; ok {
			cfg.Model = d.Model
			if d.Effort != nil {
				cfg.Effort = *d.Effort
			}
		}
	}
	if model != "" {
		cfg.Model = model
		cfg.Effort = ""
	}
	return cfg, nil
}

func Model(cfg Config) (provider.LanguageModel, error) {
	if cfg.Provider == "" || cfg.Model == "" || cfg.APIKey == "" {
		return nil, fmt.Errorf("sdk: incomplete config: provider=%q model set=%v (api key is not displayed for security)", cfg.Provider, cfg.Model != "")
	}

	switch cfg.Provider {
	case "openai":
		return openai.Chat(cfg.Model, openai.WithAPIKey(cfg.APIKey)), nil
	case "anthropic":
		return anthropic.Chat(cfg.Model, anthropic.WithAPIKey(cfg.APIKey)), nil
	case "openrouter":
		return openrouter.Chat(cfg.Model, openrouter.WithAPIKey(cfg.APIKey)), nil
	default:
		return nil, fmt.Errorf("sdk: unknown provider %q (options are openai, anthropic, or openrouter)", cfg.Provider)
	}
}

func Generate(ctx context.Context, cfg Config, system, prompt string, opts ...Option) (string, error) {
	model, err := Model(cfg)
	if err != nil {
		return "", err
	}

	gen := generateOptions{}
	for _, opt := range opts {
		opt(&gen)
	}

	goaiOpts := []goai.Option{
		goai.WithSystem(system),
		goai.WithPrompt(prompt),
	}
	if gen.temperature != 0 {
		goaiOpts = append(goaiOpts, goai.WithTemperature(gen.temperature))
	}

	if cfg.Effort != "" {
		effortOpt, err := effortOption(cfg.Provider, cfg.Effort)
		if err != nil {
			return "", err
		}
		goaiOpts = append(goaiOpts, effortOpt)
	}

	if len(gen.tools) > 0 {
		goaiOpts = append(goaiOpts, goai.WithTools(sdkTools(gen.tools)...))
		goaiOpts = append(goaiOpts, goai.WithMaxSteps(gen.maxStepLimit()))
	}

	if gen.debugLog != nil {
		goaiOpts = append(goaiOpts, debugHooks(gen.debugLog)...)
	}

	result, err := goai.GenerateText(ctx, model, goaiOpts...)

	if err != nil {
		return "", fmt.Errorf("generate text: %w", err)
	}

	if len(gen.tools) > 0 {
		keepReportStep(result)
	}

	if result.StepsExhausted {
		return "", fmt.Errorf("generation reached the maximum number of steps (%d) before completing; raise the step limit or narrow the task", gen.maxStepLimit())
	}

	return result.Text, nil
}

func debugHooks(l DebugLogger) []goai.Option {
	return []goai.Option{
		goai.WithOnStepFinish(func(step goai.StepResult) {
			var b strings.Builder
			fmt.Fprintf(&b, "step %d finished", step.Number)
			if strings.TrimSpace(step.Reasoning) != "" {
				fmt.Fprintf(&b, "\n--- step %d thinking ---\n%s", step.Number, strings.TrimSpace(step.Reasoning))
			}
			if strings.TrimSpace(step.Text) != "" {
				fmt.Fprintf(&b, "\n--- step %d text ---\n%s", step.Number, strings.TrimSpace(step.Text))
			}
			for _, tc := range step.ToolCalls {
				fmt.Fprintf(&b, "\n--- step %d tool call: %s(%s)", step.Number, tc.Name, string(tc.Input))
			}
			l.Printf("%s", b.String())
		}),
		goai.WithOnToolCall(func(info goai.ToolCallInfo) {
			if info.Error != nil {
				l.Printf("tool %s failed after %s: %v", info.ToolName, info.Duration, info.Error)
				return
			}
			l.Printf("tool %s completed in %s (output: %d bytes)", info.ToolName, info.Duration, len(info.Output))
		}),
		goai.WithOnFinish(func(info goai.FinishInfo) {
			l.Printf("generation finished: steps=%d stoppedBy=%s stepsExhausted=%v usage(in=%d out=%d total=%d)",
				info.TotalSteps, info.StoppedBy, info.StepsExhausted,
				info.TotalUsage.InputTokens, info.TotalUsage.OutputTokens, info.TotalUsage.TotalTokens)
		}),
	}
}

func keepReportStep(result *goai.TextResult) {
	for i := len(result.Steps) - 1; i >= 0; i-- {
		step := result.Steps[i]
		if len(step.ToolCalls) == 0 && strings.TrimSpace(step.Text) != "" {
			result.Text = step.Text
			return
		}
	}
}

func (o generateOptions) maxStepLimit() int {
	if o.maxSteps > 0 {
		return o.maxSteps
	}
	return defaultMaxSteps
}

func effortOption(provider, effort string) (goai.Option, error) {
	switch provider {
	case "openai", "openrouter":
		return goai.WithProviderOptions(map[string]any{"reasoning_effort": effort}), nil
	case "anthropic":
		return goai.WithProviderOptions(map[string]any{"effort": effort}), nil
	default:
		return nil, fmt.Errorf("sdk: provider %q does not support effort", provider)
	}
}

func sdkTools(tools []Tool) []goai.Tool {
	out := make([]goai.Tool, 0, len(tools))
	for _, t := range tools {
		out = append(out, goai.Tool{
			Name: t.Name,
			Description: t.Description,
			InputSchema: t.InputSchema,
			Execute: t.Execute,
		})
	}
	return out
}
