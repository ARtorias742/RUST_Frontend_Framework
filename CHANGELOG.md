# Changelog

All notable changes to Ferrum. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning is
semver from 1.0.

## [0.1.0-alpha] — unreleased

First public alpha. The architecture is in place; the API is not stable.

### Added

- `ferrum-core`: `VNode` IR, `rsx!`, `#[component]`, hooks (`use_state`,
  `use_effect`, `use_memo`, `use_callback`, `use_ref`, `use_reducer`,
  `use_context`, `use_id`, `use_custom_rc`).
- `ferrum-reconciler`: keyed diffing, hydration with mismatch recovery.
- `ferrum-web`: `web-sys` backend, `AppHandle`, browser history.
- `ferrum-native`: scene graph + accessibility tree (no window/GPU yet).
- `ferrum-testing`: headless DOM + `TestRenderer`.
- `ferrum-router`, `ferrum-data`, `ferrum-styling`, `ferrum-lazy`.
- `ferrum-components`: 40 widgets + `CATALOG`.
- `ferrum-ssr`: string/writer/stream/document rendering.
- `ferrum-devtools`: inspector, state timeline, network panel, overlay.
- `ferrum-security`: HTML sanitizer, CSP builder, no-eval lint.
- `ferrum-registry`: semver + deprecation policy.
- `ferrum`: 0.1 facade + `prelude`.
- `ferrum-cli`: `new` (4 templates), `dev` (SSE HMR), `build`
  (`--size-budget`), `check`, `doctor`, `release` (SBOM + gates).
- CI: native tests, wasm tests, Playwright, benches, fuzz, docs.
- 3 example apps (`showcase`, `dashboard`, `blog`) + 13 total examples.
- `examples/realworld`: persistent task board demo (public API only).
- GitHub Pages docs + demo, release/publish/CodeQL/Dependabot workflows.
- `website-next`: Next.js 14 + Tailwind + shadcn/ui + Framer Motion site.
- `vscode-extension`: commands, snippets, rsx! highlighting, `.vsix` package.
- `scripts/install.sh` CLI installer; local release artifacts in `dist/`.
- Ferrum UI Phase 1–4: semantic tokens + `Class`/`Style`, `ferrum-ui-core`
  (focus scope, roving tabindex, portal host), `#[component(default)]`.
- `ferrum add` source distribution: button, dialog, menu, tabs, toast;
  `--list`, `--dry-run`, `--diff`, `--force`, manifest.
- `ferrum-ui` crate: Button, Dialog, Menu, Tabs, Toaster with ARIA/keyboard.
- 10 more components (40 total) + web/native parity tests.
- Template registry with versions/deprecation; realworld uses `Dialog`.
- `GOVERNANCE.md`, `SUPPORT.md`, `deny.toml`, clippy gate.

### Known limitations

- No real native window/GPU backend.
- No concurrent rendering, error boundaries, Suspense, or transitions.
- Browser matrix and Lighthouse not verified.
- Sanitizer/CSP not independently audited; fuzzing not yet run in CI.
- Bundle is 181 KB gzip vs the 150 KB 1.0 target (alpha budget 200 KB).
- Not published to crates.io; no signed artifacts.

[0.1.0-alpha]: https://github.com/ferrum-rs/ferrum/releases/tag/v0.1.0-alpha