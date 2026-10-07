#!/usr/bin/env bash
# Regenerates crates/smetana/tests/data/routing-*.txt, the Java side of the bit-exact spline routing test
# (crates/smetana/tests/routing.rs): the calls Smetana makes to splines.c, routespl.c and the port functions of
# shapes.c, recorded by RoutingDump while it lays out
#   - routing-random.txt: random graphs (tools/oracle/smetana-trace/rockuml/oracle/RandomGraphs.java);
#   - routing-synthetic.txt: graphs with arrows PlantUML does not draw, and box corridors checkpath must repair;
#   - routing-corpus.txt: the corpus cases that use Smetana.
#
# Copies of the three classes with calls to RoutingDump (routing-hooks.patch) are compiled into their own
# directory, which comes before the golden-model jar on the class path; reference/ and the jar stay untouched.
#
# Usage: routing.sh [seed or first-last]...
# The default seeds keep the fixture small: the plain graphs 1-100, and two later ones with flat edges whose labels
# make dot route them with simpleSplineRoute.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

[[ -f "$reference_jar" ]] || die "golden model not built; run tools/oracle/build-reference.sh"

seeds=("$@")
((${#seeds[@]} > 0)) || seeds=(1-100 200 241)
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
	local fixture="$1"
	shift
	"$jdk_bin/java" -Djava.awt.headless=true -Duser.language=en -Duser.country=US -Dline.separator=$'\n' \
		-cp "$classes$separator$jar_path" rockuml.oracle.RoutingDump "$(echo "$data/$fixture" | to_native_paths)" \
		"$@" > /dev/null 2>&1
}

# RandomGraphs moves each layout's trace out of the staging directory; the traces are not needed here.
ROCKUML_SMETANA_TRACE="$(echo "$build/staging" | to_native_paths)" \
	dump routing-random.txt random "$(echo "$build/traces" | to_native_paths)" "${seeds[@]}"
dump routing-synthetic.txt synthetic 30
mapfile -t cases < <(find "$repo_root/tests/smetana" -mindepth 2 -maxdepth 2 -type d ! -path '*/random/*' \
	| sed "s|^$repo_root/tests/smetana/|$corpus_dir/|; s|$|.puml|" | sort)
dump routing-corpus.txt corpus $(printf '%s\n' "${cases[@]}" | to_native_paths)
for fixture in "$data"/routing-*.txt; do
	echo "$(grep -c '^end$' "$fixture") calls in $fixture."
done
