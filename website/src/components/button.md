# Button

Copy it into your app:

```bash
ferrum add button
```

```rust
use ferrum::prelude::*;
use ferrum_ui::Button;

rsx! {
    Button { variant: "primary", disabled: false, on_click: None, "Save" }
}
```

## Variants

`primary` (default) and `ghost`. `disabled` removes the click handler and
sets the `disabled` attribute. The focus ring uses `:focus-visible`.

## Accessibility

- Native `<button>` semantics.
- `disabled` is a real attribute, not just a class.
- Icon-only buttons must pass an `aria-label` (use the `children` block with
  an `aria-label` on a wrapping element, or extend the template).

## Source

The template is embedded in `ferrum add`; run `ferrum add --dry-run button`
to print it before writing.
