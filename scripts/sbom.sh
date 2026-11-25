#!/usr/bin/env bash
# Generate a CycloneDX SBOM for the workspace (1.0 release gate).
set -euo pipefail
VERSION="${1:-0.1.0}"
cargo run -q -p ferrum-cli -- release --version "$VERSION" --sbom-only
