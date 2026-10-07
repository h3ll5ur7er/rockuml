#!/usr/bin/env bash
# Records Smetana layout traces of reproducible random graphs (tools/oracle/smetana-trace/rockuml/oracle/RandomGraphs.java)
# into tests/smetana/random/<seed>.trace, with one line per seed in tests/smetana/random/summary.txt: its style, size
# and features, or why it was skipped.
#
# Usage: smetana-random.sh [seeds]   (default 300; seeds run from 1)
# BATCH sets how many seeds one JVM lays out (default 50). Traces do not depend on it.
set -euo pipefail

source "$(dirname "$0")/common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

seeds="${1:-300}"
batch="${BATCH:-50}"
random_dir="$repo_root/tests/smetana/random"
staging_root="$oracle_dir/build/random-staging"

# Each JVM gets its own staging directory, since SmetanaTrace numbers its files per JVM.
trace_batch() {
	local first="$1" last="$2" staging="$staging_root/$1"
	mkdir -p "$staging/traces"
	ROCKUML_SMETANA_TRACE="$(echo "$staging/traces" | to_native_paths)" "$jdk_bin/java" -Djava.awt.headless=true \
		-cp "$(echo "$reference_jar" | to_native_paths)" rockuml.oracle.RandomGraphs \
		"$(echo "$random_dir" | to_native_paths)" "$first" "$last" > "$staging/summary.txt" 2> "$staging/stderr.txt" \
		|| { cat "$staging/stderr.txt" >&2; exit 1; }
}

rm -rf "$random_dir" "$staging_root"
mkdir -p "$random_dir"

export -f trace_batch to_native_paths
export jdk_bin reference_jar random_dir staging_root

for ((first = 1; first <= seeds; first += batch)); do
	echo "$first $((first + batch - 1 < seeds ? first + batch - 1 : seeds))"
done | xargs -P "$(nproc)" -L 1 bash -c 'trace_batch "$1" "$2"' _ || die "a batch failed"

cat "$staging_root"/*/summary.txt | sort > "$random_dir/summary.txt"
rm -rf "$staging_root"
echo "$(grep -c ' ok ' "$random_dir/summary.txt") traces, $(grep -c ' skipped ' "$random_dir/summary.txt") skipped," \
	"in $random_dir."
