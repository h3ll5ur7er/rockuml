#!/usr/bin/env bash
# Regenerates the golden outputs for every case in tests/corpus (or for the given .puml files).
#
# Each case's outputs go to <case>.golden/ beside it. A dedicated directory per case is needed because
# a block can name its own output (`@startuml other-name`), so names alone don't tell which case
# produced a file.
set -euo pipefail

source "$(dirname "$0")/common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

list_cases() {
	if (($# > 0)); then
		printf '%s\0' "$@"
	else
		find "$corpus_dir" -name '*.puml' -print0
	fi
}

generate_case() {
	local case_file="$1"
	local golden_dir
	golden_dir="$(realpath -m "${case_file%.puml}.golden")"
	local native_golden_dir
	rm -rf "$golden_dir"
	mkdir -p "$golden_dir"
	native_golden_dir="$(echo "$golden_dir" | to_native_paths)"
	# Deliberate syntax errors make PlantUML exit non-zero; their error image is still the golden.
	reference_plantuml -preproc -o "$native_golden_dir" "$case_file" > /dev/null 2>&1 || true
	reference_plantuml -f debug -o "$native_golden_dir" "$case_file" > /dev/null 2>&1 || true
	reference_plantuml -f svg -o "$native_golden_dir" "$case_file" > /dev/null 2>&1 || true
	reference_plantuml -encodeurl "$case_file" > "$golden_dir/$(basename "${case_file%.puml}").url" 2> /dev/null || true
	mask_render_timestamps "$golden_dir"
}

# The debug format stamps the current time next to shapes it cannot describe. Masking it keeps
# regenerated goldens stable; the parity test masks rockuml's output the same way.
mask_render_timestamps() {
	find "$1" -name '*.debug' -exec sed -i -E \
		's/(Mon|Tue|Wed|Thu|Fri|Sat|Sun) [A-Z][a-z]{2} [0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2} [^ ]+ [0-9]{4}/<timestamp>/g' {} +
}

export -f generate_case mask_render_timestamps reference_plantuml to_native_paths
export jdk_bin reference_jar

list_cases "$@" | xargs -0 -P "$(nproc)" -I {} bash -c 'generate_case "$1"' _ {}

echo "Goldens generated for $(list_cases "$@" | tr -cd '\0' | wc -c) cases."
