#!/usr/bin/env bash
# Builds the golden-model jar from the PlantUML reference sources.
#
# The LGPL source drop references TeaVM, OpenPDF and Ant, none of which ship with it.
# Compile-only stubs (tools/oracle/stubs) stand in for them: TeaVM code is unreachable
# on a JVM, and PDF/Ant are outside the oracle's scope.
#
# Graphviz's bundled Windows dot.exe is deliberately left out of the jar so that
# PlantUML falls back to Smetana, the layout engine rockuml ports.
set -euo pipefail

source "$(dirname "$0")/common.sh"

reference_sources="$repo_root/reference/plantuml-lgpl-1.2026.8-sources"
stubs="$oracle_dir/stubs"
build="$oracle_dir/build"

[[ -d "$reference_sources" ]] || die "reference sources not found at $reference_sources"

rm -rf "$build/classes"
mkdir -p "$build/classes"

find "$reference_sources" -name '*.java' \
	| grep -v -E '/plantuml/(ant|openpdf)/' \
	| to_native_paths > "$build/sources.txt"
echo "$reference_sources/net/sourceforge/plantuml/openpdf/PdfOption.java" | to_native_paths >> "$build/sources.txt"
find "$stubs" -name '*.java' | to_native_paths > "$build/stubs.txt"

echo "Compiling $(wc -l < "$build/sources.txt") reference sources..."
"$jdk_bin/javac" -nowarn -encoding UTF-8 -d "$build/classes" "@$build/sources.txt" "@$build/stubs.txt" \
	> "$build/javac.log" 2>&1 \
	|| { cat "$build/javac.log"; die "javac failed"; }

echo "Copying resources..."
(cd "$reference_sources" \
	&& find . -type f ! -name '*.java' ! -path './META-INF/*' ! -name 'graphviz.dat' \
	| tar cf - -T - ) | (cd "$build/classes" && tar xf -)

printf 'Main-Class: net.sourceforge.plantuml.Run\n' > "$build/manifest.txt"
"$jdk_bin/jar" cfm "$reference_jar" "$build/manifest.txt" -C "$build/classes" .
echo "Built $reference_jar"
