#!/usr/bin/env bash
# Dumps Java's cgraph state after replaying the input section of every Smetana trace, for the port's cgraph tests
# (crates/smetana/tests/cgraph_replay.rs).
#
# tests/smetana/<area>/<case>/NN.trace gives tests/smetana-cgraph/<area>/<case>/NN.dump; the hand-written inputs
# in tests/smetana-cgraph/synthetic/*.trace get their dump next to them.
set -euo pipefail

source "$(dirname "$0")/common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

traces_dir="$repo_root/tests/smetana"
dumps_dir="$repo_root/tests/smetana-cgraph"
classes_dir="$oracle_dir/build/cgraph-dump"

mkdir -p "$classes_dir"
"$jdk_bin/javac" -d "$(echo "$classes_dir" | to_native_paths)" -cp "$(echo "$reference_jar" | to_native_paths)" \
	"$(echo "$oracle_dir/cgraph-dump/CgraphDump.java" | to_native_paths)"
classpath="$(echo "$reference_jar" | to_native_paths)$(if command -v cygpath > /dev/null; then echo ';'; else echo ':'; fi)$(echo "$classes_dir" | to_native_paths)"

dump() {
	mkdir -p "$(dirname "$2")"
	"$jdk_bin/java" -cp "$classpath" CgraphDump "$(echo "$1" | to_native_paths)" "$(echo "$2" | to_native_paths)"
}

find "$dumps_dir" -name '*.dump' ! -path "$dumps_dir/synthetic/*" -delete 2> /dev/null || true
count=0
while IFS= read -r -d '' trace; do
	relative="$(realpath --relative-to="$traces_dir" "$trace")"
	dump "$trace" "$dumps_dir/${relative%.trace}.dump"
	count=$((count + 1))
done < <(find "$traces_dir" -name '*.trace' -print0)
while IFS= read -r -d '' input; do
	dump "$input" "${input%.trace}.dump"
	count=$((count + 1))
done < <(find "$dumps_dir/synthetic" -name '*.trace' -print0)
echo "$count cgraph dumps in $dumps_dir."
