# Contributing

Ferrum is a 0.1 alpha. The most valuable contributions make the alpha more
credible: browser tests, a real native backend, docs, and release plumbing.

```bash
rustup target add wasm32-unknown-unknown
cargo test --workspace
cargo fmt --check
```

- Keep `ferrum-core` backend-agnostic.
- No `unsafe` without justification.
- Update tests and `CHANGELOG.md` in the same PR.
- Discuss public API changes in an issue first.

See the repository `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, and
`SECURITY.md` for the full policy.