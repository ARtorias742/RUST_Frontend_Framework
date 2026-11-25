# Architecture

## Crates

| Crate | Role |
|---|---|
| `ferrum-core` | `VNode` IR, hooks, events, macros |
| `ferrum-reconciler` | Keyed diffing, hydration, `Renderer` trait |
| `ferrum-web` | `web-sys` backend + `AppHandle` |
| `ferrum-native` | Scene graph + a11y tree |
| `ferrum-testing` | Headless DOM + `TestRenderer` |
| `ferrum-router` | Routes, params, redirects, layouts |
| `ferrum-data` | Query cache, dedup, `use_query` |
| `ferrum-styling` | Scoped CSS, tokens, themes |
| `ferrum-components` | 30 widgets + `CATALOG` |
| `ferrum-ssr` | String/writer/stream/document |
| `ferrum-lazy` | Code-splitting boundary |
| `ferrum-devtools` | Inspector, timeline, network |
| `ferrum-security` | Sanitizer, CSP, no-eval |
| `ferrum-registry` | Semver + deprecation |
| `ferrum` | Facade + `prelude` |

## Rules

- `ferrum-core` knows nothing about a backend.
- Backends implement `ferrum-reconciler::Renderer`.
- Components are plain functions; hooks are call-order slots.
- Effects flush after commit; cleanups run on unmount.
- The facade is the only stability boundary.

## Rendering pipeline

1. `rsx!` produces a `VNode` tree.
2. `TreeRenderer` mounts/updates it through a `Renderer` backend.
3. Keyed children are matched by `plan_children`.
4. Components get an `Instance` (stable id + hook slots).
5. State setters schedule a re-render; effects flush after commit.