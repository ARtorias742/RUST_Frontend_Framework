#!/usr/bin/env bash
# Enforce the 1.0 bundle budget: core runtime < 150 KB gzipped.
# Usage: scripts/check-size.sh [path-to.wasm] [kb-budget]
set -euo pipefail
WASM="${1:-target/wasm32-unknown-unknown/release/example_showcase.wasm}"
BUDGET="${2:-150}"
if [ ! -f "$WASM" ]; then
  echo "error: $WASM not found (run: cargo build --release --target wasm32-unknown-unknown -p example-showcase)" >&2
  exit 1
fi
RAW=$(stat -c%s "$WASM")
GZ=$(gzip -c "$WASM" | wc -c)
RAW_KB=$((RAW / 1024))
GZ_KB=$((GZ / 1024))
echo "wasm: ${RAW_KB} KB raw, ${GZ_KB} KB gzipped (budget ${BUDGET} KB)"
if [ "$GZ_KB" -gt "$BUDGET" ]; then
  echo "error: gzipped wasm exceeds the ${BUDGET} KB budget" >&2
  exit 1
fi
