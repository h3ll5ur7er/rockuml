#!/usr/bin/env bash
# Regenerates crates/rockuml/tests/data/activity-notes-groups.txt, the Java side of the exact note and group tile
# test (crates/rockuml/src/diagram/activity3/tests/notes_groups.rs), by running NoteGroupDump against the
# golden-model jar.
set -euo pipefail

source "$(dirname "$0")/../common.sh"

fixture="$repo_root/crates/rockuml/tests/data/activity-notes-groups.txt"
mkdir -p "$(dirname "$fixture")"
run_dump activity-unit/NoteGroupDump "$(native_path "$fixture")"
echo "$(grep -c '^=== ' "$fixture") cases in $fixture."
