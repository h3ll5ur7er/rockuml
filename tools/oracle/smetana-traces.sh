#!/usr/bin/env bash
# Records the Smetana layout traces of every corpus case (or of the given .puml files) for Phase 4's layout oracle.
#
# Each case is rendered with `-f debug` and ROCKUML_SMETANA_TRACE set, so every Smetana layout it runs writes
# tests/smetana/<area>/<case>/NN.trace (format in tools/oracle/README.md). Cases that lay nothing out with Smetana
# get no directory.
set -euo pipefail

source "$(dirname "$0")/common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

traces_dir="$repo_root/tests/smetana"

list_cases() {
	if (($# > 0)); then
		printf '%s\0' "$@"
	else
		find "$corpus_dir" -name '*.puml' -print0
	fi
}

trace_case() {
	local case_file trace_dir
	case_file="$(realpath "$1")"
	trace_dir="$traces_dir/$(realpath --relative-to="$corpus_dir" "${case_file%.puml}")"
	rm -rf "$trace_dir"
	mkdir -p "$trace_dir/.render"
	ROCKUML_SMETANA_TRACE="$(echo "$trace_dir" | to_native_paths)" \
		reference_plantuml -f debug -o "$(echo "$trace_dir/.render" | to_native_paths)" "$case_file" > /dev/null 2>&1 || true
	rm -rf "$trace_dir/.render"
	rmdir --ignore-fail-on-non-empty "$trace_dir"
}

# A full run starts afresh, so traces of removed cases do not linger. The random graphs' traces are
# smetana-random.sh's.
(($# > 0)) || find "$traces_dir" -mindepth 1 -maxdepth 1 ! -name random -exec rm -rf {} + 2> /dev/null || true

export -f trace_case reference_plantuml to_native_paths
export jdk_bin reference_jar corpus_dir traces_dir

list_cases "$@" | xargs -0 -P "$(nproc)" -I {} bash -c 'trace_case "$1"' _ {}

find "$traces_dir" -type d -empty -delete 2> /dev/null || true
echo "$(find "$traces_dir" -path "$traces_dir/random" -prune -o -name '*.trace' -print 2> /dev/null | wc -l) traces in $traces_dir."
