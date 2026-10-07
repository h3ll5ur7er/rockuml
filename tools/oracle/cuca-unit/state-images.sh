#!/usr/bin/env bash
# Regenerates crates/rockuml/tests/data/state-images.txt, the Java side of the exact state image test
# (crates/rockuml/src/diagram/state/image_tests.rs), by running StateImageDump against the golden-model jar.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

classes="$oracle_dir/build/cuca-unit"
fixture="$repo_root/crates/rockuml/tests/data/state-images.txt"

mkdir -p "$classes" "$(dirname "$fixture")"
jar_path="$(echo "$reference_jar" | to_native_paths)"
classes_path="$(echo "$classes" | to_native_paths)"
"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$jar_path" -d "$classes_path" \
	"$(echo "$oracle_dir/cuca-unit/StateImageDump.java" | to_native_paths)"

# java.exe on Windows wants ';' between classpath entries.
separator=":"
[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
# A fixed line separator keeps the fixture's bytes the same on every platform.
"$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Dline.separator=$'\n' \
	-cp "$jar_path$separator$classes_path" StateImageDump "$(echo "$fixture" | to_native_paths)" 2> /dev/null
echo "$(grep -c '^=== ' "$fixture") images in $fixture."
