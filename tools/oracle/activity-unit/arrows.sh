#!/usr/bin/env bash
# Regenerates crates/rockuml/tests/data/activity-arrows.txt, the Java side of the exact arrow and tile geometry
# test (crates/rockuml/src/ftile/tests/arrows.rs), by running ArrowDump against the golden-model jar.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

fixture="$repo_root/crates/rockuml/tests/data/activity-arrows.txt"
mkdir -p "$(dirname "$fixture")"
run_dump activity-unit/ArrowDump "$(native_path "$fixture")"
echo "$(grep -c '^merge\|^mutation\|^geometry' "$fixture") cases in $fixture."
