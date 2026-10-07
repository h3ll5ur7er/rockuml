#!/usr/bin/env bash
# Regenerates crates/rockuml/tests/data/activity-arrows.txt, the Java side of the exact arrow and tile geometry
# test (crates/rockuml/src/ftile/tests/arrows.rs), by running ArrowDump against the golden-model jar.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

classes="$oracle_dir/build/activity-unit"
fixture="$repo_root/crates/rockuml/tests/data/activity-arrows.txt"

mkdir -p "$classes" "$(dirname "$fixture")"
jar_path="$(echo "$reference_jar" | to_native_paths)"
classes_path="$(echo "$classes" | to_native_paths)"
"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$jar_path" -d "$classes_path" \
	"$(echo "$oracle_dir/activity-unit/ArrowDump.java" | to_native_paths)"

# java.exe on Windows wants ';' between classpath entries.
separator=":"
[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
# A fixed line separator keeps the fixture's bytes the same on every platform.
"$jdk_bin/java" -Djava.awt.headless=true -Dline.separator=$'\n' -cp "$jar_path$separator$classes_path" ArrowDump \
	"$(echo "$fixture" | to_native_paths)" 2> /dev/null
echo "$(grep -c '^merge\|^mutation\|^geometry' "$fixture") cases in $fixture."
