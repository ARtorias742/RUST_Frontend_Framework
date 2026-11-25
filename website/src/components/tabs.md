# Tabs

```bash
ferrum add tabs
```

```rust
use ferrum_ui::{Tabs, TabItem};

let items = vec![
    TabItem { label: "One".into(), content: rsx! { p { "first" } } },
    TabItem { label: "Two".into(), content: rsx! { p { "second" } } },
];
rsx! { Tabs { orientation: "horizontal", active: 0, on_change: None, items: items } }
```

## Keyboard

- `role="tablist"`, `role="tab"`, `role="tabpanel"`.
- `aria-selected`, `aria-controls`, `aria-labelledby`.
- Roving tabindex; only the active tab is tabbable.
- Arrow keys, Home/End and RTL flipping are host responsibilities.

## Lazy panels

The template renders only the active panel. For expensive panels, render on
first activation and cache.
