#!/usr/bin/env bash
# Builds rockuml.wasm and puts it next to the JavaScript shim in web/.
# Usage: bash tools/build-wasm.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo build --profile wasm-release --target wasm32-unknown-unknown -p rockuml-wasm
cp target/wasm32-unknown-unknown/wasm-release/rockuml_wasm.wasm web/rockuml.wasm
echo "web/rockuml.wasm: $(wc -c < web/rockuml.wasm) bytes"
