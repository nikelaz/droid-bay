#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/dist"
PLATFORMS=(
	linux/amd64
	linux/arm64
	darwin/amd64
	darwin/arm64
	windows/amd64
)

rm -rf "$DIST"
mkdir -p "$DIST/agents" "$DIST/skills"

for agent_dir in "$ROOT"/agents/*/; do
	name="$(basename "$agent_dir")"
	if [ ! -f "$agent_dir/go.mod" ]; then
		continue
	fi

	for platform in "${PLATFORMS[@]}"; do
		os="${platform%/*}"
		arch="${platform#*/}"
		out_dir="$DIST/agents/$name/$os-$arch"
		mkdir -p "$out_dir"
		binary="$name"
		if [ "$os" = "windows" ]; then
			binary="$name.exe"
		fi
		(cd "$agent_dir" && GOOS="$os" GOARCH="$arch" CGO_ENABLED=0 go build -trimpath -o "$out_dir/$binary" .)
		echo "built agents/$name/$os-$arch/$binary"
	done
done

if [ -d "$ROOT/skills" ]; then
	find "$ROOT/skills" -mindepth 1 -maxdepth 1 -exec cp -r {} "$DIST/skills/" \;
fi

echo "release ready at $DIST"
