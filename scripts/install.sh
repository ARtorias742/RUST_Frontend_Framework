#!/usr/bin/env bash
# Install the Ferrum CLI from source (until 0.1 is on crates.io).
# Usage: scripts/install.sh [git-url]
set -euo pipefail
REPO="${1:-${FERRUM_REPO:-https://github.com/ferrum-rs/ferrum}}"
echo "installing ferrum CLI from $REPO"
cargo install --git "$REPO" --bin ferrum --locked
