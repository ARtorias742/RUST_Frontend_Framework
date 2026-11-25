#!/usr/bin/env bash
# The public-alpha gate. Everything here must pass before tagging a release.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== fmt =="
cargo fmt --check

echo "== native tests =="
cargo test --workspace

echo "== conformance =="
cargo test -p ferrum-conformance -- --nocapture

echo "== release wasm + size =="
cargo build --release --target wasm32-unknown-unknown -p example-showcase
scripts/check-size.sh

echo "== benches =="
cargo bench -p ferrum-bench

echo "alpha gate passed"
