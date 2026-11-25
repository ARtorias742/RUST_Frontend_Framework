#!/usr/bin/env bash
# Alpha budget: core runtime < 200 KB gzipped (1.0 target: 150 KB).
# Usage: scripts/check-size.sh [path-to.wasm] [kb-budget]
set -euo pipefail
WASM="${1:-target/wasm32-unknown-unknown/release/example_counter_web.wasm}"
BUDGET="${2:-200}"
if [ ! -f "$WASM" ]; then
  echo "error: $WASM not found (run: cargo build --release --target wasm32-unknown-unknown -p example-showcase)" >&2
  exit 1
fi
RAW=$(stat -c%s "$WASM")
GZ=$(gzip -c "$WASM" | wc -c)
RAW_KB=$((RAW / 1024))
GZ_KB=$((GZ / 1024))
echo "wasm: ${RAW_KB} KB raw, ${GZ_KB} KB gzipped (alpha budget ${BUDGET} KB; 1.0 target 150 KB)"
if [ "$GZ_KB" -gt "$BUDGET" ]; then
  echo "error: gzipped wasm exceeds the ${BUDGET} KB budget" >&2
  exit 1
fi
