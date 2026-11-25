# Ferrum

> **0.1.0-alpha. Not production-ready.**

Ferrum is a Rust frontend framework with a React-shaped API: `rsx!` +
`#[component]`, hooks, keyed reconciliation, SSR/hydration, routing, data,
scoped styling, 40 components, DevTools, and a native scene-graph backend.

The architecture is in place. The API is not stable. This alpha exists to
validate the design with real applications before 1.0.

- [Status](status.md)
- [Quickstart](quickstart.md)
- [Architecture](architecture.md)
- [Testing](testing.md)

## Design goals

1. **Small, explicit core.** A tiny VNode IR; renderers are replaceable.
2. **React-shaped, not React-compatible.** Familiar hooks and components,
   Rust semantics.
3. **Safe by default.** No `eval`, escaped SSR, allowlist sanitizer, CSP.
4. **One component, many backends.** Web, headless, and native share the
   reconciler.

## Why Ferrum

- Rust end to end, one type system, no JS bridge in component code.
- A small explicit core with replaceable renderers.
- Safe by default: no `eval`, escaped SSR, allowlist sanitizer, nonce CSP.
- Router, data, styling, 40 components, DevTools and SSR in the crate graph.

## Demo

The alpha demo is a persistent task board built with the public API only:
[`examples/realworld`](quickstart.md). It is deployed at `/demo/` on GitHub
Pages.

## Non-goals (for 1.0)

- A full React clone (concurrent rendering, Suspense, transitions).
- A native window/GPU stack (the scene graph is the contract; the window is
  a platform adapter).
- A component library as large as a design system.