#!/usr/bin/env bash
# Runs the golden-model PlantUML with the given arguments, e.g. `reference-plantuml.sh -f debug foo.puml`.
set -euo pipefail

source "$(dirname "$0")/common.sh"

reference_plantuml "$@"
