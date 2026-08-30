package helpers

import (
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"time"
)

func SetupRunLog(dir string) (func(), error) {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return nil, fmt.Errorf("create log dir: %w", err)
	}

	f, err := os.OpenFile(filepath.Join(dir, time.Now().Format("20060102-150405")+".log"),
		os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0o644)
	if err != nil {
		return nil, fmt.Errorf("create log file: %w", err)
	}

	log.SetOutput(io.MultiWriter(os.Stderr, f))

	return func() { f.Close() }, nil
}
