#!/usr/bin/env bash
# Run the browser (wasm-bindgen-test) suite headlessly.
# Requires: wasm-pack, Chrome and/or Firefox.
set -euo pipefail
if ! command -v wasm-pack >/dev/null; then
  echo "error: wasm-pack is required (cargo install wasm-pack)" >&2
  exit 1
fi
rustup target add wasm32-unknown-unknown
wasm-pack test --headless --chrome --firefox crates/ferrum-web
