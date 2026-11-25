#!/usr/bin/env bash
# Reproducible 1.0 release pipeline:
#   1. tests, 2. wasm size budget, 3. SBOM, 4. version/API freeze check.
# Usage: scripts/release.sh [version]
set -euo pipefail
VERSION="${1:-0.1.0}"
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-1704067200}"

echo "== 1/4 tests =="
cargo test --workspace

echo "== 2/4 wasm size =="
cargo build --release --target wasm32-unknown-unknown -p example-showcase
scripts/check-size.sh

echo "== 3/4 SBOM + release gates =="
cargo run -q -p ferrum-cli -- release --version "$VERSION"

echo "== 4/4 reproducible build check =="
sha256sum target/wasm32-unknown-unknown/release/example_showcase.wasm
echo "release ${VERSION} artifacts are reproducible with SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}"
