package helpers

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

func CloneRepo(owner, repo, root string) (string, error) {
	dir := filepath.Join(root, owner, repo)
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