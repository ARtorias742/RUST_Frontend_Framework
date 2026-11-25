# Contributing to Ferrum

Ferrum is a public **0.1 alpha**. The architecture is stable enough to build
on; the API is not. Contributions that make the alpha more credible are the
most valuable right now.

## Ground rules

- Be respectful. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
- Discuss large changes in an issue first (especially public API changes).
- Keep the renderer-agnostic boundary: `ferrum-core` must not depend on a
  backend, and backends must not depend on each other.
- No `unsafe` in the facade or core; justify any `unsafe` in a comment.
- Update tests and docs in the same PR.

## Development

```bash
rustup target add wasm32-unknown-unknown
cargo test --workspace
cargo test -p ferrum-conformance
cargo bench -p ferrum-bench
```

Browser and E2E suites:

```bash
scripts/wasm-test.sh   # needs wasm-pack + Chrome/Firefox
scripts/e2e.sh         # needs trunk + Playwright
```

## What to work on

Good first issues:

- A real native backend spike (winit + wgpu + AccessKit).
- More conformance checks (error boundaries, Suspense-like boundaries).
- Browser E2E coverage and visual baselines.
- Docs site and tutorials.
- crates.io release plumbing and signed artifacts.

## Pull requests

1. Fork and create a branch.
2. Keep the diff focused; one logical change per PR.
3. Add or update tests.
4. Run `cargo test --workspace` and `cargo fmt --check`.
5. Describe the user-visible change and any API impact.

## API policy (0.1)

- Breaking changes are allowed in minor releases.
- Every breaking change needs a `CHANGELOG.md` entry and a migration note.
- The `ferrum` facade is the only stability boundary; internal crates may
  change freely.