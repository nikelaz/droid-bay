package helpers

import (
	"os"
	"path/filepath"
)

func PathFor(name string) string {
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