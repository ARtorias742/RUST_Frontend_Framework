# Status

| Area | State |
|---|---|
| Core IR, `rsx!`, components, hooks | Working, tested natively |
| Keyed reconciler + hydration | Working, tested |
| Web renderer | Implemented; wasm tests in CI |
| SSR | Working, tested |
| Router + data | Working, tested |
| Styling + 30 components | Working, headless-tested |
| DevTools | Working |
| Native renderer | Scene graph + a11y tree only |
| Security | Basic, not audited |
| Browser matrix / Lighthouse | Not verified |
| API stability | None (0.1 alpha) |
| Bundle | 181 KB gzip (alpha budget 200 KB; 1.0 target 150 KB) |

## What “alpha” means here

- The architecture and crate boundaries are stable.
- Public APIs may change in any 0.1.x release.
- Some documented features are spikes, not production backends.
- Performance numbers are release-mode local benchmarks, not a guarantee.

## Release cadence

Until 1.0: small, frequent alpha releases. Every release has a
`CHANGELOG.md` entry and a migration note for breaking changes.