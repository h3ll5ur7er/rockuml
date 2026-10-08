#!/usr/bin/env bash
# Prints the tile tree PlantUML builds for activity corpus cases, with each tile's geometry, by running
# FtileDump against the golden-model jar. A debugging aid for porting tiles.
# Usage: tools/oracle/activity-unit/ftiles.sh activity/start-stop.puml [more cases...]
set -euo pipefail

source "$(dirname "$0")/../common.sh"

(($# > 0)) || die "usage: $0 <case relative to tests/corpus>..."

run_dump activity-unit/FtileDump "$(native_path "$corpus_dir")" "$@"
