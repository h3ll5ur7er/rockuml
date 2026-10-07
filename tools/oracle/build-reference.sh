#!/usr/bin/env bash
# Builds the golden-model jar from the PlantUML reference sources.
#
# The LGPL source drop references TeaVM, OpenPDF and Ant, none of which ship with it.
# Compile-only stubs (tools/oracle/stubs) stand in for them: TeaVM code is unreachable
# on a JVM, and PDF/Ant are outside the oracle's scope.
#
# The jar starts through a small launcher (tools/oracle/launcher) that turns off the minute-dependent
# donation banners of error images, which would make goldens depend on when they were generated.
#
# Graphviz's bundled Windows dot.exe is deliberately left out of the jar so that
# PlantUML falls back to Smetana, the layout engine rockuml ports.
#
# tools/oracle/smetana-trace/hooks.patch adds calls to a layout tracer (SmetanaTrace) to copies of a few
# Smetana sources; reference/ itself stays untouched. The tracer is inert unless ROCKUML_SMETANA_TRACE is set.
set -euo pipefail

source "$(dirname "$0")/common.sh"

reference_sources="$repo_root/reference/plantuml-lgpl-1.2026.8-sources"
stubs="$oracle_dir/stubs"
smetana_trace="$oracle_dir/smetana-trace"
build="$oracle_dir/build"
patched_sources="$build/patched-sources"

[[ -d "$reference_sources" ]] || die "reference sources not found at $reference_sources"

rm -rf "$build/classes" "$patched_sources"
mkdir -p "$build/classes" "$patched_sources"

sed -n 's|^+++ b/||p' "$smetana_trace/hooks.patch" > "$build/patched.txt"
while read -r source; do
	mkdir -p "$patched_sources/$(dirname "$source")"
	cp "$reference_sources/$source" "$patched_sources/$source"
done < "$build/patched.txt"
patch --quiet -p1 -d "$patched_sources" < "$smetana_trace/hooks.patch" || die "hooks.patch does not apply"

find "$reference_sources" -name '*.java' \
	| grep -v -E '/plantuml/(ant|openpdf)/' \
	| grep -v -x -F -f <(sed "s|^|$reference_sources/|" "$build/patched.txt") \
	| to_native_paths > "$build/sources.txt"
echo "$reference_sources/net/sourceforge/plantuml/openpdf/PdfOption.java" | to_native_paths >> "$build/sources.txt"
find "$patched_sources" -name '*.java' | to_native_paths >> "$build/sources.txt"
find "$stubs" "$oracle_dir/launcher" "$smetana_trace" -name '*.java' | to_native_paths > "$build/stubs.txt"

echo "Compiling $(wc -l < "$build/sources.txt") reference sources..."
"$jdk_bin/javac" -nowarn -encoding UTF-8 -d "$build/classes" "@$build/sources.txt" "@$build/stubs.txt" \
	> "$build/javac.log" 2>&1 \
	|| { cat "$build/javac.log"; die "javac failed"; }

echo "Copying resources..."
(cd "$reference_sources" \
	&& find . -type f ! -name '*.java' ! -path './META-INF/*' ! -name 'graphviz.dat' \
	| tar cf - -T - ) | (cd "$build/classes" && tar xf -)

printf 'Main-Class: rockuml.oracle.OracleMain\n' > "$build/manifest.txt"
"$jdk_bin/jar" cfm "$reference_jar" "$build/manifest.txt" -C "$build/classes" .
echo "Built $reference_jar"
