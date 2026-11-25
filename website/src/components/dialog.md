# Dialog

```bash
ferrum add dialog
```

```rust
use ferrum_ui::Dialog;

rsx! {
    Dialog { open: true, title: "Confirm", on_close: None, "Body" }
}
```

## Behavior

- Renders nothing when `open` is false.
- `role="dialog"`, `aria-modal`, `aria-label`.
- Close button is optional; without `on_close` it is decorative.
- The headless `FocusScope` in `ferrum-ui-core` is the focus-trap model; the
  host wires real focus ids.

## Accessibility checklist

- [ ] Focus moves into the dialog on open.
- [ ] Tab/Shift+Tab cycle inside.
- [ ] Escape closes.
- [ ] Focus returns to the trigger.
- [ ] Background is `aria-hidden`/inert.

The template implements the markup; the focus behavior is provided by
`ferrum-ui-core` and the host.
