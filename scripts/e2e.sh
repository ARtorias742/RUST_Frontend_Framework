#!/usr/bin/env bash
# Build the DevTools example and run the Playwright suite against it.
# Requires: trunk, node/npm, Playwright browsers.
set -euo pipefail
if ! command -v trunk >/dev/null; then
  echo "error: trunk is required (cargo install trunk)" >&2
  exit 1
fi
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/examples/devtools"
trunk build --dist "$ROOT/target/e2e"
cd "$ROOT/target/e2e"
python3 -m http.server 8080 &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT
sleep 1
cd "$ROOT/e2e"
npm install
npx playwright install --with-deps
FERRUM_BASE_URL=http://localhost:8080 npm run test:devtools
