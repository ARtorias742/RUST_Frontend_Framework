# Tutorial: the realworld task board

The demo app is `examples/realworld`. It uses only the public API.

```bash
cd examples/realworld
trunk serve
```

## What it shows

- `use_state` for tasks, draft and filter.
- `use_effect_every` for `localStorage` persistence.
- `ferrum-components`: `Card`, `Input`, `Button`, `Checkbox`, `EmptyState`.
- `ferrum-ui`: `Dialog` for the About panel.
- The same component source renders on web and native.

## Add a component

```bash
ferrum add dialog
```

The source lands in `src/ui/dialog.rs`; edit it freely. Run
`ferrum add --diff dialog` to see what changed after an upgrade.
