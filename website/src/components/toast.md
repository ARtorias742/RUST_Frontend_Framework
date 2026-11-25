# Toast

```bash
ferrum add toast
```

```rust
use ferrum_ui::{Toast, ToastLevel, Toaster};

let toasts = vec![Toast {
    id: 1,
    level: ToastLevel::Success,
    title: "Saved".into(),
    description: "ok".into(),
}];
rsx! { Toaster { toasts: toasts, on_dismiss: None } }
```

## Behavior

- The host owns the queue and max-visible policy.
- `aria-live="polite"` region; errors use `role="alert"`.
- Pause auto-dismiss on hover/focus (host effect).
- Timers live in effects, never during render (SSR-safe).
