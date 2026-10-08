#!/usr/bin/env bash
# Regenerates crates/rockuml/tests/data/activity-compress.txt, the Java side of the exact compression test
# (crates/rockuml/src/klimt/compress/tests.rs), by running CompressDump against the golden-model jar.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

fixture="$repo_root/crates/rockuml/tests/data/activity-compress.txt"
mkdir -p "$(dirname "$fixture")"
run_dump activity-unit/CompressDump "$(native_path "$fixture")"
echo "$(grep -c '^scene' "$fixture") scenes in $fixture."
