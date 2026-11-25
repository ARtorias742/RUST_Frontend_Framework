# Menu

```bash
ferrum add menu
```

```rust
use ferrum_ui::{Menu, MenuItem};

let items = vec![MenuItem { label: "One".into(), disabled: false, on_select: None }];
rsx! { Menu { open: true, active: 0, items: items, button { "Open" } } }
```

## Keyboard

- `RovingTabIndex` gives one tab stop (`0` active, `-1` others).
- ArrowUp/Down, Home/End, Enter/Space and Escape are host responsibilities.
- Typeahead is a host helper; the menu exposes the item list.

## Accessibility

`role="menu"` / `role="menuitem"`, `aria-disabled`, trigger
`aria-expanded`/`aria-haspopup` (added by the host).
