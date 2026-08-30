package sdk

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/nikelaz/droid-bay/helpers"
)

const (
	searxngContainer = "droid-bay-searxng"
	defaultSearxngImage = "docker.io/searxng/searxng:latest"
	defaultSearxngPort = 8888
	searxngInternalPort = "8080/tcp"
	searxngStartTimeout = 3 * time.Minute
	searxngReadyTimeout = 30 * time.Second
	searchHTTPTimeout = 30 * time.Second
	maxSearchResults = 15
)

func WebSearchTools(ctx context.Context) ([]Tool, func(), error) {
	if raw := os.Getenv("SEARXNG_URL"); raw != "" {
		inst := &searxngInstance{baseURL: strings.TrimRight(raw, "/")}
		if err := inst.waitReady(ctx, searxngReadyTimeout); err != nil {
			return nil, nil, fmt.Errorf("sdk: searxng at %s: %w", inst.baseURL, err)
		}
		return []Tool{searxngTool(inst)}, func() {}, nil
	}

	inst, err := startManagedSearxng(ctx)
	if err != nil {
		return nil, nil, err
	}
	return []Tool{searxngTool(inst)}, inst.stop, nil
}

type searxngInstance struct {
	baseURL string
	managed bool
	runtime string
	stopOnce sync.Once
}

func (s *searxngInstance) stop() {
	s.stopOnce.Do(func() {
		if !s.managed {
			return
		}
		exec.Command(s.runtime, "stop", "-t", "10", searxngContainer).Run()
	})
}

func startManagedSearxng(ctx context.Context) (*searxngInstance, error) {
	runtime, err := containerRuntime()
	if err != nil {
		return nil, err
	}

	if containerRunning(runtime) {
		if hostPort, portErr := publishedPort(runtime); portErr == nil {
			inst := &searxngInstance{baseURL: "http://127.0.0.1:" + hostPort, managed: true, runtime: runtime}
			if err := inst.waitReady(ctx, searxngReadyTimeout); err == nil {
				return inst, nil
			}
		}
		removeContainer(runtime)
	} else {
		removeContainer(runtime)
	}

	hostPort, err := chooseHostPort(defaultSearxngPort)
	if err != nil {
		return nil, err
	}

	settingsPath, err := writeSearxngSettings()
	if err != nil {
		return nil, err
	}

	baseURL := fmt.Sprintf("http://127.0.0.1:%d", hostPort)
	image := helpers.EnvOr("SEARXNG_IMAGE", defaultSearxngImage)
	out, err := exec.CommandContext(ctx, runtime, "run", "-d",
		"--name", searxngContainer,
		"--rm",
		"-p", fmt.Sprintf("127.0.0.1:%d:8080", hostPort),
		"-v", settingsPath+":/etc/searxng/settings.yml:ro",
		"-e", "SEARXNG_BASE_URL="+baseURL+"/",
		image,
	).CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("sdk: start %s container: %w: %s", runtime, err, strings.TrimSpace(string(out)))
	}

	inst := &searxngInstance{baseURL: baseURL, managed: true, runtime: runtime}
	if err := inst.waitReady(ctx, searxngStartTimeout); err != nil {
		logs, logErr := exec.Command(runtime, "logs", "--tail", "20", searxngContainer).CombinedOutput()
		inst.stop()
		if logErr == nil && strings.TrimSpace(string(logs)) != "" {
			return nil, fmt.Errorf("sdk: searxng container did not become ready: %w (logs: %s)", err, strings.TrimSpace(string(logs)))
		}
		return nil, fmt.Errorf("sdk: searxng container did not become ready: %w", err)
	}

	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, os.Interrupt, syscall.SIGTERM)
	go func() {
		<-sigs
		inst.stop()
		os.Exit(130)
	}()

	return inst, nil
}

func containerRuntime() (string, error) {
	for _, name := range []string{"podman", "docker"} {
		if path, err := exec.LookPath(name); err == nil {
			return path, nil
		}
	}
	return "", fmt.Errorf("sdk: install podman (or docker) to run the bundled SearXNG container, or set SEARXNG_URL to an existing SearXNG instance")
}

func containerRunning(runtime string) bool {
	out, err := exec.Command(runtime, "ps", "--filter", "name="+searxngContainer, "--format", "{{.Names}}").Output()
	return err == nil && strings.Contains(string(out), searxngContainer)
}

func publishedPort(runtime string) (string, error) {
	out, err := exec.Command(runtime, "port", searxngContainer, searxngInternalPort).Output()
	if err != nil {
		return "", err
	}
	for _, field := range strings.Fields(string(out)) {
		if _, port, err := net.SplitHostPort(field); err == nil && port != "" {
			return port, nil
		}
	}
	return "", fmt.Errorf("sdk: container %s has no published port", searxngContainer)
}

func removeContainer(runtime string) {
	exec.Command(runtime, "rm", "-f", searxngContainer).Run()
}

func chooseHostPort(preferred int) (int, error) {
	for port := preferred; port < preferred+50; port++ {
		listener, err := net.Listen("tcp", fmt.Sprintf("127.0.0.1:%d", port))
		if err == nil {
			listener.Close()
			return port, nil
		}
	}
	return 0, fmt.Errorf("sdk: no free port at or above %d for searxng", preferred)
}

func writeSearxngSettings() (string, error) {
	root, err := os.UserCacheDir()
	if err != nil {
		root = os.TempDir()
	}
	dir := filepath.Join(root, "droid-bay", "searxng")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return "", err
	}
	var key [16]byte
	if _, err := rand.Read(key[:]); err != nil {
		return "", err
	}
	settings := fmt.Sprintf(`use_default_settings: true
server:
  secret_key: %s
  limiter: false
search:
  formats:
    - html
    - json
`, hex.EncodeToString(key[:]))
	path := filepath.Join(dir, "settings.yml")
	if err := os.WriteFile(path, []byte(settings), 0o644); err != nil {
		return "", err
	}
	return path, nil
}

func (s *searxngInstance) waitReady(ctx context.Context, timeout time.Duration) error {
	client := &http.Client{Timeout: 5 * time.Second}
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
		case <-time.After(500 * time.Millisecond):
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
