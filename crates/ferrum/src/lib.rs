//! # Ferrum
//!
//! A Rust frontend framework with a React-shaped API: [`rsx!`] + `#[component]`,
//! hooks, a keyed reconciler, SSR/hydration, a native renderer, routing,
//! data fetching, scoped styling and a 30-component library.
//!
//! This crate is the **1.0 facade**: it re-exports every public API so apps
//! can depend on `ferrum` alone. The facade is the stability boundary — the
//! internal crates may be refactored without breaking users.
//!
//! ```ignore
//! use ferrum::prelude::*;
//!
//! #[component]
//! fn Counter() -> VNode {
//!     let (count, set) = use_state(|| 0i32);
//!     rsx! { button { onclick: move |_| set.update(|c| c + 1), "Count: {count}" } }
//! }
//! ```
//!
//! ## Feature map
//!
//! | Crate | Purpose |
//! |---|---|
//! | `ferrum-core` | VNode IR, hooks runtime, event types, `rsx!`/`#[component]` |
//! | `ferrum-reconciler` | Renderer-agnostic keyed diffing + hydration |
//! | `ferrum-web` | `web-sys` backend + `AppHandle` |
//! | `ferrum-native` | Scene-graph backend + accessibility tree |
//! | `ferrum-testing` | Headless DOM + `TestRenderer` |
//! | `ferrum-router` | Declarative routes, params, redirects, layouts |
//! | `ferrum-data` | Query cache, dedup, `use_query` |
//! | `ferrum-styling` | Scoped CSS, tokens, themes, Tailwind interop |
//! | `ferrum-components` | 30 headless-styled widgets |
//! | `ferrum-ssr` | String/writer/stream/document rendering |
//! | `ferrum-lazy` | Code-splitting boundary |
//! | `ferrum-devtools` | Inspector, state timeline, network panel, overlay |
//! | `ferrum-security` | HTML sanitizer, CSP, no-eval lint |
//! | `ferrum-registry` | Semver + deprecation policy |

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use ferrum_components::*;
pub use ferrum_core::*;
pub use ferrum_data::*;
pub use ferrum_devtools::*;
pub use ferrum_lazy::*;
pub use ferrum_native::*;
pub use ferrum_reconciler::*;
pub use ferrum_registry::*;
pub use ferrum_router::*;
pub use ferrum_security::*;
pub use ferrum_ssr::*;
pub use ferrum_styling::*;
pub use ferrum_testing::*;
pub use ferrum_web::*;

/// The recommended import for applications.
pub mod prelude {
    pub use crate::{
        component, rsx, use_callback, use_context, use_effect, use_effect_every, use_id, use_memo,
        use_reducer, use_ref, use_state, AttrValue, Context, FerrumEvent, VNode,
    };
    pub use ferrum_components::*;
    pub use ferrum_styling::{css, theme_context, tw, Theme};
}

/// The crate version (1.0 API freeze).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The compatibility gate enforced by `ferrum-conformance`.
pub const CONFORMANCE_GATE: f64 = ferrum_conformance::PASS_RATIO_GATE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facade_reexports_the_core_api() {
        let node: VNode = rsx! { div { "hello" } };
        assert_eq!(ferrum_ssr::render_to_string(&node), "<div>hello</div>");
    }

    #[test]
    fn version_is_1_0() {
        assert_eq!(VERSION, "0.1.0");
        assert!(CONFORMANCE_GATE >= 0.95);
    }
}