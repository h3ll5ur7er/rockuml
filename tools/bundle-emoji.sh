#!/usr/bin/env bash
# Rebuilds crates/rockuml/assets/emoji/twemoji.br from PlantUML's Twemoji set (emoji/data in the reference sources).
# Usage: bash tools/bundle-emoji.sh [reference sources directory, by default reference/plantuml-lgpl-1.2026.8-sources]
#
# One record per line, in emoji.txt's order: `<code>[;<shortcut>] <svg>`. PlantUML reads only the first line of each
# SVG file. Brotli keeps the 1.7 MB of SVG text near 0.5 MB in the binary and the repository.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
reference="${1:-$repo_root/reference/plantuml-lgpl-1.2026.8-sources}"
data="$reference/net/sourceforge/plantuml/emoji/data"
target="$repo_root/crates/rockuml/assets/emoji/twemoji.br"

command -v brotli > /dev/null || { echo "error: needs the brotli command" >&2; exit 1; }
mkdir -p "$(dirname "$target")"
while IFS= read -r entry || [[ -n "$entry" ]]; do
	svg="$(head -n 1 "$data/${entry%%;*}.svg")"
	printf '%s %s\n' "$entry" "$svg"
done < "$data/emoji.txt" | brotli --best --large_window=24 --force --output="$target"
