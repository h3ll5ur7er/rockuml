#!/usr/bin/env bash
# Regenerates crates/smetana/tests/data/pathplan.txt, the Java side of the bit-exact path planner test
# (crates/smetana/tests/pathplan.rs), by running PathplanDump against the golden-model jar.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

classes="$oracle_dir/build/smetana-unit"
fixture="$repo_root/crates/smetana/tests/data/pathplan.txt"

rm -rf "$classes"
mkdir -p "$classes" "$(dirname "$fixture")"
jar_path="$(echo "$reference_jar" | to_native_paths)"
classes_path="$(echo "$classes" | to_native_paths)"
"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$jar_path" -d "$classes_path" \
	"$(echo "$oracle_dir/smetana-unit/PathplanDump.java" | to_native_paths)"

# java.exe on Windows wants ';' between classpath entries.
separator=":"
[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
# A fixed line separator keeps the fixture's bytes the same on every platform.
"$jdk_bin/java" -Dline.separator=$'\n' -cp "$jar_path$separator$classes_path" PathplanDump \
	"$(echo "$fixture" | to_native_paths)" 2> /dev/null
echo "$(grep -c '^case\|^solve3' "$fixture") cases in $fixture."
