//! # ferrum-styling
//!
//! Scoped CSS + design tokens + theming.
//!
//! - [`css!`] / [`css`] register a CSS rule and return a deterministic
//!   scoped class name (hash of the rule), so styles never collide.
//! - [`stylesheet`] renders the whole registry (injected into the document
//!   by `ferrum-web` on startup).
//! - [`Theme`] holds design tokens; themes are provided via a
//!   [`Context<Theme>`] ([`theme_context`] / [`use_theme`]) and applied as
//!   CSS custom properties by [`ThemeRoot`] (in `ferrum-components`), so
//!   components style themselves with `var(--token)` — no recompilation
//!   needed to switch themes (dark mode, RTL via `dir`).

use ferrum_core::{use_context, Context};
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Scoped CSS
// ---------------------------------------------------------------------------

thread_local! {
    static REGISTRY: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
    static VERSION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// A scoped class name (e.g. `f1a2b3c4`). Derefs to `str`.
#[derive(Clone, Debug)]
pub struct ScopedClass(String);

impl ScopedClass {
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl Deref for ScopedClass {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ScopedClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Register a CSS rule and return its scoped class name.
///
/// ```ignore
/// let cls = css!("color: red; padding: 4px;");
/// rsx! { div { class: "{cls}", "hi" } }
/// ```
pub fn css(rule: &str) -> ScopedClass {
    let mut hasher = DefaultHasher::new();
    rule.hash(&mut hasher);
    let name = format!("f{:x}", hasher.finish());
    REGISTRY.with(|r| {
        let mut reg = r.borrow_mut();
        if !reg.iter().any(|(n, _)| *n == name) {
            reg.push((name.clone(), rule.to_string()));
            VERSION.with(|v| v.set(v.get() + 1));
        }
    });
    ScopedClass(name)
}

/// The full stylesheet for all registered rules.
pub fn stylesheet() -> String {
    REGISTRY.with(|r| {
        r.borrow()
            .iter()
            .map(|(n, css)| format!(".{}{{{}}}", n, css))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Registry version (bumps when a new rule is registered).
pub fn registry_version() -> u64 {
    VERSION.with(|v| v.get())
}

/// Clear the registry (tests).
pub fn reset_registry() {
    REGISTRY.with(|r| r.borrow_mut().clear());
    VERSION.with(|v| v.set(0));
}

/// Register a CSS rule with a scoped class. `css! { color: red; }` is
/// sugar for `css("color : red ;")` (whitespace is CSS-tolerant).
#[macro_export]
macro_rules! css {
    ($($t:tt)*) => {
        $crate::css(stringify!($($t)*))
    };
}

// ---------------------------------------------------------------------------
// Tailwind interoperability
// ---------------------------------------------------------------------------

/// Join utility classes (e.g. Tailwind) into one `class` attribute value.
///
/// Tailwind is a build-time CSS generator, not a runtime dependency: it
/// scans source files for class *strings* and emits the matching CSS. This
/// helper keeps those strings ergonomic while remaining compatible with the
/// scoped `css!` classes — both are just `class` tokens:
///
/// ```ignore
/// rsx! { div { class: tw!("flex", "items-center", gap), "hi" } }
/// ```
///
/// Empty/`None` entries are skipped, so conditional classes compose cleanly.
/// If a class is built dynamically (not a literal in the source), add it to
/// Tailwind's `safelist` so the scanner can see it.
pub fn cx(classes: &[&str]) -> String {
    let mut out = String::new();
    for class in classes {
        if class.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(class);
    }
    out
}

/// Join Tailwind/utility classes into a single `class` value.
///
/// `tw!("px-4", "py-2")` → `"px-4 py-2"`. Literal strings are visible to
/// Tailwind's source scanner; dynamic values should be safelisted. Use an
/// empty string for a conditional class: `tw!("flex", if x { "gap-2" } else { "" })`.
#[macro_export]
macro_rules! tw {
    ($($class:expr),* $(,)?) => {
        $crate::cx(&[$($class),*])
    };
}

// ---------------------------------------------------------------------------
// Themes (design tokens)
// ---------------------------------------------------------------------------

/// Design tokens. Applied as CSS custom properties by `ThemeRoot`.
#[derive(Clone, Debug)]
pub struct Theme {
    pub bg: String,
    pub fg: String,
    pub accent: String,
    pub muted: String,
    pub radius: String,
    /// Text direction: "ltr" or "rtl".
    pub dir: &'static str,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::light()
    }
}

impl Theme {
    pub fn light() -> Self {
        Theme {
            bg: "#ffffff".into(),
            fg: "#111827".into(),
            accent: "#2563eb".into(),
            muted: "#6b7280".into(),
            radius: "8px".into(),
            dir: "ltr",
        }
    }

    pub fn dark() -> Self {
        Theme {
            bg: "#111827".into(),
            fg: "#f9fafb".into(),
            accent: "#60a5fa".into(),
            muted: "#9ca3af".into(),
            radius: "8px".into(),
            dir: "ltr",
        }
    }

    pub fn rtl(mut self) -> Self {
        self.dir = "rtl";
        self
    }

    /// The theme as CSS custom-property declarations (for the `style` attr).
    pub fn style_string(&self) -> String {
        format!(
            "--bg: {}; --fg: {}; --accent: {}; --muted: {}; --radius: {};",
            self.bg, self.fg, self.accent, self.muted, self.radius
        )
    }
}

thread_local! {
    static THEME_CTX: std::cell::OnceCell<Context<Theme>> = const { std::cell::OnceCell::new() };
}

/// The global theme context (stable id).
pub fn theme_context() -> Context<Theme> {
    THEME_CTX.with(|c| c.get_or_init(Context::new).clone())
}

/// `useTheme`: the nearest theme (panics without a provider).
pub fn use_theme() -> Theme {
    use_context(&theme_context())
}

/// Convenience: an `Rc` shared theme for app state.
pub type SharedTheme = Rc<Theme>;
