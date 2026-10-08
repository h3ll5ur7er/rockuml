#!/usr/bin/env bash
# Regenerates crates/rockuml/tests/data/activity-model.txt, the Java side of the exact activity model test
# (crates/rockuml/src/diagram/activity3/tests.rs), by running ModelDump on the activity corpus.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

fixture="$repo_root/crates/rockuml/tests/data/activity-model.txt"
mkdir -p "$(dirname "$fixture")"

cases=()
for file in "$corpus_dir"/activity/*.puml; do
	cases+=("activity/$(basename "$file")")
done

run_dump activity-unit/ModelDump "$(native_path "$fixture")" "$(native_path "$corpus_dir")" "${cases[@]}"
echo "$(grep -c '^=== ' "$fixture") cases in $fixture."
