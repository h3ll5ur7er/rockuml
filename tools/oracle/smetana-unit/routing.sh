#!/usr/bin/env bash
# Regenerates crates/smetana/tests/data/routing-*.txt, the Java side of the bit-exact spline routing test
# (crates/smetana/tests/routing.rs): the calls Smetana makes to splines.c, routespl.c and the port functions of
# shapes.c while laying out the random graphs and the corpus cases, recorded by RoutingDump.
#
# Copies of the three classes with calls to RoutingDump (routing-hooks.patch) are compiled into their own
# directory, which comes before the golden-model jar on the class path; reference/ and the jar stay untouched.
#
# Usage: routing.sh [seeds]   (random graphs 1 to seeds, default 300)
set -euo pipefail

source "$(dirname "$0")/../common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

seeds="${1:-300}"
reference_sources="$repo_root/reference/plantuml-lgpl-1.2026.8-sources"
unit="$oracle_dir/smetana-unit"
build="$oracle_dir/build/smetana-routing"
data="$repo_root/crates/smetana/tests/data"

rm -rf "$build"
mkdir -p "$build/sources" "$build/classes" "$build/staging" "$build/traces" "$data"
sed -n 's|^+++ b/||p' "$unit/routing-hooks.patch" | while read -r source; do
	mkdir -p "$build/sources/$(dirname "$source")"
	cp "$reference_sources/$source" "$build/sources/$source"
done
patch --quiet -p1 -d "$build/sources" < "$unit/routing-hooks.patch" || die "routing-hooks.patch does not apply"

jar_path="$(echo "$reference_jar" | to_native_paths)"
classes="$(echo "$build/classes" | to_native_paths)"
"$jdk_bin/javac" -nowarn -encoding UTF-8 -cp "$jar_path" -d "$classes" \
	$(find "$build/sources" "$unit/RoutingDump.java" -name '*.java' | to_native_paths)

# java.exe on Windows wants ';' between classpath entries.
separator=":"
[[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* || "$(uname -s)" == CYGWIN* ]] && separator=";"
dump() {
	"$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Dline.separator=$'\n' \
		-cp "$classes$separator$jar_path" rockuml.oracle.RoutingDump "$@" > /dev/null 2>&1
}

ROCKUML_SMETANA_TRACE="$(echo "$build/staging" | to_native_paths)" \
	dump "$(echo "$data/routing-random.txt" | to_native_paths)" random 1 "$seeds" \
	"$(echo "$build/traces" | to_native_paths)"
mapfile -t cases < <(find "$repo_root/tests/smetana" -mindepth 2 -maxdepth 2 -type d ! -path '*/random/*' \
	| sed "s|^$repo_root/tests/smetana/|$corpus_dir/|; s|$|.puml|" | sort)
dump "$(echo "$data/routing-corpus.txt" | to_native_paths)" corpus $(printf '%s\n' "${cases[@]}" | to_native_paths)
for fixture in "$data"/routing-*.txt; do
	echo "$(grep -c '^end$' "$fixture") calls in $fixture."
done
