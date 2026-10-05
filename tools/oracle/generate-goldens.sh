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
}

export -f generate_case reference_plantuml to_native_paths
export jdk_bin reference_jar

list_cases "$@" | xargs -0 -P "$(nproc)" -I {} bash -c 'generate_case "$1"' _ {}

echo "Goldens generated for $(list_cases "$@" | tr -cd '\0' | wc -c) cases."
