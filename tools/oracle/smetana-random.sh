#!/usr/bin/env bash
# Records Smetana layout traces of reproducible random graphs (tools/oracle/smetana-trace/rockuml/oracle/RandomGraphs.java)
# into tests/smetana/random/<seed>.trace, with one line per seed in tests/smetana/random/summary.txt: its style, size
# and features, or why it was skipped.
#
# Usage: smetana-random.sh [seeds]   (default 300; seeds run from 1)
# The seeds listed in tests/smetana/random/extra-seeds.txt (one per line, '#' starts a comment) are added: higher
# seeds kept because they reach code the first ones miss.
# BATCH sets how many seeds one JVM lays out (default 50). Traces do not depend on it.
# SEED_TIMEOUT (seconds, default 120) bounds each layout; a seed that exceeds it is recorded as skipped.
set -euo pipefail

source "$(dirname "$0")/common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

seeds="${1:-300}"
batch="${BATCH:-50}"
seed_timeout="${SEED_TIMEOUT:-120}"
random_dir="$repo_root/tests/smetana/random"
extra_seeds="$random_dir/extra-seeds.txt"
staging_root="$oracle_dir/build/random-staging"
# RandomGraphs' exit status after a seed timed out.
timed_out=3

# Lays out the given seeds, one JVM at a time: after a timeout the JVM exits, and the next one resumes with the
# seeds after the one that timed out. Each batch gets its own staging directory, since SmetanaTrace numbers its
# files per JVM.
trace_batch() {
	local staging="$staging_root/$1" status last
	shift
	local remaining=("$@")
	mkdir -p "$staging/traces"
	while ((${#remaining[@]})); do
		status=0
		ROCKUML_SMETANA_TRACE="$(echo "$staging/traces" | to_native_paths)" "$jdk_bin/java" -Djava.awt.headless=true \
			-cp "$(echo "$reference_jar" | to_native_paths)" rockuml.oracle.RandomGraphs \
			"$(echo "$random_dir" | to_native_paths)" "$seed_timeout" "${remaining[@]}" \
			>> "$staging/summary.txt" 2>> "$staging/stderr.txt" || status=$?
		((status == 0)) && return
		((status == timed_out)) || { cat "$staging/stderr.txt" >&2; exit 1; }
		last="$(tail -n 1 "$staging/summary.txt" | cut -d ' ' -f 1)"
		echo "seed $((10#$last)) timed out after $seed_timeout s" >&2
		rm -f "$staging/traces"/*
		while ((10#${remaining[0]} != 10#$last)); do remaining=("${remaining[@]:1}"); done
		remaining=("${remaining[@]:1}")
	done
}

list_seeds() {
	seq 1 "$seeds"
	if [[ -f "$extra_seeds" ]]; then
		sed 's/#.*//' "$extra_seeds" | tr -s ' \t\r' '\n\n\n'
	fi
}
all_seeds="$(list_seeds | grep . | sort -n -u)"

rm -rf "$staging_root"
mkdir -p "$random_dir"
rm -f "$random_dir"/*.trace "$random_dir/summary.txt"

export -f trace_batch to_native_paths
export jdk_bin reference_jar random_dir staging_root seed_timeout timed_out

echo "$all_seeds" | xargs -n "$batch" | nl -n rz -w 4 | xargs -P "$(nproc)" -L 1 bash -c 'trace_batch "$@"' _ \
	|| die "a batch failed"

cat "$staging_root"/*/summary.txt | sort -n > "$random_dir/summary.txt"
rm -rf "$staging_root"
echo "$(grep -c ' ok ' "$random_dir/summary.txt") traces, $(grep -c ' skipped ' "$random_dir/summary.txt") skipped," \
	"in $random_dir."
