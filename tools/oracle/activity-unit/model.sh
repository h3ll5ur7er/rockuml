#!/usr/bin/env bash
# Regenerates crates/rockuml/tests/data/activity-model.txt, the Java side of the exact activity model test
# (crates/rockuml/src/diagram/activity3/tests.rs), by running ModelDump on the activity corpus.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

classes="$oracle_dir/build/activity-unit"
fixture="$repo_root/crates/rockuml/tests/data/activity-model.txt"

mkdir -p "$classes" "$(dirname "$fixture")"
jar_path="$(echo "$reference_jar" | to_native_paths)"
classes_path="$(echo "$classes" | to_native_paths)"
"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$jar_path" -d "$classes_path" \
	"$(echo "$oracle_dir/activity-unit/ModelDump.java" | to_native_paths)"

cases=()
for file in "$corpus_dir"/activity/*.puml; do
	cases+=("activity/$(basename "$file")")
done

# java.exe on Windows wants ';' between classpath entries.
separator=":"
[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
# A fixed line separator keeps the fixture's bytes the same on every platform.
"$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Dline.separator=$'\n' \
	-cp "$jar_path$separator$classes_path" ModelDump "$(echo "$fixture" | to_native_paths)" \
	"$(echo "$corpus_dir" | to_native_paths)" "${cases[@]}" 2> /dev/null
echo "$(grep -c '^=== ' "$fixture") cases in $fixture."
