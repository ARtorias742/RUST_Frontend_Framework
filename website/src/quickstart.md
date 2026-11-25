# Quickstart

The crates are not on crates.io yet. Clone the repo and run an example.

```bash
git clone https://github.com/ferrum-rs/ferrum
cd ferrum

# Native debug render (no browser)
cargo run -p example-counter

# Browser
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve examples/counter-web
```

## A component

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

## Examples

| Example | Shows |
|---|---|
| `counter` | IR + hooks, no DOM |
| `counter-web` | counter in the browser |
| `todo` | keyed list, controlled input |
| `form` | form + validation |
| `hooks` | every hook |
| `spa` | routes, nested layout, redirect, async data |
| `ssr` | styled HTML document |
| `native` | scene graph + a11y tree |
| `devtools` | DevTools overlay |
| `showcase` | component gallery |
| `dashboard` | tables + progress |
| `blog` | routed blog with params |

## `ferrum new`

The CLI can scaffold `blank`, `spa`, `dashboard`, and `ssr` templates, but
the generated manifests reference the not-yet-published crates. Use the
examples until 0.1 is on crates.io.