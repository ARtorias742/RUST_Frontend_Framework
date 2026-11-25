# Testing

```bash
cargo test --workspace
cargo test -p ferrum-conformance -- --nocapture
cargo bench -p ferrum-bench
```

## Browser

```bash
scripts/wasm-test.sh   # wasm-pack + Chrome/Firefox
scripts/e2e.sh         # Playwright + trunk
```

## Gates

| Gate | Target |
|---|---|
| Conformance | ≥ 95% (currently 20/20) |
| Keyed update | < 16.6 ms per 1k children |
| Startup | < 1.5 s TTI |
| Soak | ≤ 5% heap growth |
| Bundle | < 200 KB gzip (alpha); 1.0 target 150 KB |

## Fuzzing

```bash
scripts/fuzz.sh sanitize 60
```

Targets: `sanitize`, `ssr`, `reconciler`. Requires nightly + cargo-fuzz.