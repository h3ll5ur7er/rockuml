#!/usr/bin/env bash
# Collects the license texts that rockuml's web and npm packages carry, at the paths NOTICE.md and
# THIRD-PARTY.md link to. Usage: bash tools/collect-licenses.sh [directory, legal/ unless given]
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-$root/legal}"

mkdir -p "$out/crates/smetana" "$out/crates/rockuml/assets/fonts"
cp "$root"/{LICENSE,COPYING,NOTICE.md,THIRD-PARTY.md} "$out/"
cp "$root/crates/smetana/LICENSE" "$out/crates/smetana/"
cp "$root/crates/rockuml/assets/fonts/LICENSE-Liberation.txt" "$out/crates/rockuml/assets/fonts/"
# Windows has python, other systems python3.
python="$(command -v python3 >/dev/null && python3 --version >/dev/null 2>&1 && echo python3 || echo python)"
(cd "$root" && "$python" tools/third-party-licenses.py "$out/THIRD-PARTY-CRATES.md")
