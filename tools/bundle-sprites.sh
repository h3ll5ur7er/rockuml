#!/usr/bin/env bash
# Rebuilds crates/rockuml/assets/sprites/sprites.br from PlantUML's built-in sprites (sprites/ in the reference
# sources), which `<$archimate/actor>` and `sprite $name jar:archimate/actor` draw.
# Usage: bash tools/bundle-sprites.sh [reference sources directory, by default reference/plantuml-lgpl-1.2026.8-sources]
#
# One record per file, sorted by path: the path below sprites/, a newline, the length in bytes, a newline, then the
# file as is (SVG text or PNG). Brotli keeps the 211 KB of files near 24 KB in the binary and the repository.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
reference="${1:-$repo_root/reference/plantuml-lgpl-1.2026.8-sources}"
target="$repo_root/crates/rockuml/assets/sprites/sprites.br"

command -v brotli > /dev/null || { echo "error: needs the brotli command" >&2; exit 1; }
mkdir -p "$(dirname "$target")"
cd "$reference/sprites"
find . -type f | sed 's|^\./||' | LC_ALL=C sort | while IFS= read -r path; do
	printf '%s\n%s\n' "$path" "$(wc -c < "$path" | tr -d ' ')"
	cat "$path"
done | brotli --best --large_window=24 --force --output="$target"
