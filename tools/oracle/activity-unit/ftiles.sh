#!/usr/bin/env bash
# Prints the tile tree PlantUML builds for activity corpus cases, with each tile's geometry, by running
# FtileDump against the golden-model jar. A debugging aid for porting tiles.
# Usage: tools/oracle/activity-unit/ftiles.sh activity/start-stop.puml [more cases...]
set -euo pipefail

source "$(dirname "$0")/../common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"
(($# > 0)) || die "usage: $0 <case relative to tests/corpus>..."

classes="$oracle_dir/build/activity-unit"

mkdir -p "$classes"
jar_path="$(echo "$reference_jar" | to_native_paths)"
classes_path="$(echo "$classes" | to_native_paths)"
"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$jar_path" -d "$classes_path" \
	"$(echo "$oracle_dir/activity-unit/FtileDump.java" | to_native_paths)"

# java.exe on Windows wants ';' between classpath entries.
separator=":"
[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
"$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Dline.separator=$'\n' \
	-cp "$jar_path$separator$classes_path" FtileDump "$(echo "$corpus_dir" | to_native_paths)" "$@" 2> /dev/null
