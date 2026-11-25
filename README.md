# Ferrum

> **Alpha — 0.1.0. Not production-ready. Do not ship to end users yet.**
>
> Ferrum is a Rust frontend framework with a React-shaped API: `rsx!` +
> `#[component]`, hooks, keyed reconciliation, SSR/hydration, routing, data,
> scoped styling, a 30-component library, DevTools, and a native scene-graph
> backend. This is a **public alpha** to validate the architecture and API
> with real users. Expect breaking changes before 1.0.

Ferrum is developed in the open. The goal is a small, explicit, safe
alternative to JS-first frameworks — not a React clone.

## Status

| Area | State |
|---|---|
| Core IR, `rsx!`, components, hooks | Working, tested natively |
| Keyed reconciler + hydration | Working, tested |
| Web renderer (`web-sys`) | Implemented; wasm tests are now in CI |
| SSR (string/stream/document) | Working, tested |
| Router + data (`use_query`) | Working, tested |
| Styling + themes + 30 components | Working, headless-tested |
| DevTools (inspector/timeline/network) | Working |
| Native renderer | **Scene graph + a11y tree only** — no window/GPU yet |
| Security (sanitizer/CSP) | Basic, **not audited** |
| Browser matrix / Lighthouse | **Not verified** |
| API stability | **None** — 0.1 alpha |
| Bundle | 192 KB gzip demo (core 181 KB; alpha budget 200 KB; 1.0 target 150 KB) |

## Install

The crates are not on crates.io yet. Three options:

```bash
# 1. CLI from source (scaffold, dev server, build, release)
scripts/install.sh
# or: cargo install --git https://github.com/ferrum-rs/ferrum --bin ferrum --locked

# 2. VS Code extension: download the .vsix from Releases,
#    then Extensions → ⋯ → Install from VSIX

# 3. Cargo (once 0.1 is published)
cargo add ferrum
```

## Quickstart

The published crates are not on crates.io yet, so use the workspace examples:

```bash
git clone https://github.com/ferrum-rs/ferrum
cd ferrum

# Native debug render (no browser needed)
cargo run -p example-counter

# Browser
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve examples/counter-web
```

A minimal component:

```rust
use ferrum::prelude::*;

#[component]
fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        div {
            h1 { "Ferrum" }
            button { onclick: move |_| set.update(|c| c + 1), "Count: {count}" }
        }
    }
}
```

More examples: `examples/counter`, `todo`, `form`, `hooks`, `spa`, `ssr`,
`native`, `devtools`, `showcase`, `dashboard`, `blog`, `realworld`.

The live alpha demo (task board with `localStorage`) and the docs site are
deployed from `main` to GitHub Pages: `/demo/` and `/docs/`.

## Website

The marketing site is a Next.js 14 + Tailwind + shadcn/ui + Framer Motion app:

```bash
cd website-next
npm install
npm run dev      # http://localhost:3000
```

## VS Code extension

```bash
cd vscode-extension
npm install
npm run build
npm run package  # -> ferrum-0.1.0.vsix
```

Install the VSIX from the Extensions panel, or
`code --install-extension ferrum-0.1.0.vsix`.

## Architecture

| Crate | Role |
|---|---|
| `ferrum-core` | `VNode` IR, hooks runtime, events, `rsx!`/`#[component]` |
| `ferrum-reconciler` | Renderer-agnostic keyed diffing + hydration |
| `ferrum-web` | `web-sys` backend + `AppHandle` |
| `ferrum-native` | Scene graph + accessibility tree |
| `ferrum-testing` | Headless DOM + `TestRenderer` |
| `ferrum-router` | Routes, params, redirects, nested layouts |
| `ferrum-data` | Query cache, dedup, `use_query` |
| `ferrum-styling` | Scoped CSS, design tokens, themes |
| `ferrum-components` | 30 widgets + `CATALOG` |
| `ferrum-ssr` | String/writer/stream/document rendering |
| `ferrum-lazy` | Code-splitting boundary |
| `ferrum-devtools` | Inspector, state timeline, network panel |
| `ferrum-security` | HTML sanitizer, CSP, no-eval lint |
| `ferrum-registry` | Semver + deprecation policy |
| `ferrum` | 0.1 facade + `prelude` |

## Testing

```bash
cargo test --workspace            # native suite
cargo test -p ferrum-conformance  # 20-check React-subset gate
cargo bench -p ferrum-bench       # reconcile / startup / soak
scripts/check-size.sh             # gzip wasm budget (after a release build)
```

Headless browser tests and Playwright need extra tooling:

```bash
scripts/wasm-test.sh              # wasm-pack + Chrome/Firefox
scripts/e2e.sh                    # Playwright + a local server
```

## Known gaps

- No real native window/GPU backend (winit/wgpu/AccessKit).
- No concurrent rendering, error boundaries, Suspense, or transitions.
- No Safari/Firefox/Edge verification yet.
- Sanitizer and CSP are not independently audited.
- Fuzzing is configured but not yet run in CI.
- Bundle is above the 1.0 target (192 KB demo vs 150 KB gzip); optimization is pending.
- No crates.io release, docs site, or signed artifacts yet.
- `ferrum new` templates target the future published crates; use the
  examples until 0.1 is published.

## Why Ferrum

- **Rust end to end.** One language, one type system, no JS bridge in your
  component code.
- **Small explicit core.** A tiny VNode IR; web, native and headless are
  replaceable backends over one reconciler.
- **Safe by default.** No `eval`, escaped SSR, allowlist sanitizer, nonce CSP.
- **Batteries for the 80%.** Router, data, styling, 30 components, DevTools,
  SSR — without leaving the Rust crate graph.

It is not trying to be React. It is trying to be the smallest Rust framework
that can build a real app without a JS toolchain.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md),
[GOVERNANCE.md](GOVERNANCE.md), [SUPPORT.md](SUPPORT.md), and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Security reports:
[SECURITY.md](SECURITY.md).

## Release

```bash
scripts/alpha-check.sh
# build wasm + SBOM + checksums into dist/
git tag -a v0.1.0-alpha -m "Ferrum 0.1.0-alpha"
git push origin main --follow-tags
```

Pushing the tag triggers `.github/workflows/release.yml` (GitHub release,
SBOM, checksums). Publishing to crates.io is a separate workflow that needs
`CARGO_REGISTRY_TOKEN`.

## License

Apache-2.0. See [LICENSE](LICENSE).