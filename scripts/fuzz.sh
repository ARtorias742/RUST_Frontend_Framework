#!/usr/bin/env bash
# Run one cargo-fuzz target for a fixed time.
# Usage: scripts/fuzz.sh [target] [seconds]
# Requires: nightly + cargo-fuzz.
set -euo pipefail
TARGET="${1:-sanitize}"
SECONDS_RUN="${2:-60}"
cd "$(dirname "$0")/../fuzz"
cargo fuzz run "$TARGET" -- -max_total_time="$SECONDS_RUN"
