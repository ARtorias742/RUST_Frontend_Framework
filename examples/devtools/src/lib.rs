//! Phase 9 example: DevTools overlay over a small interactive app.
//!
//! The overlay is a snapshot view of the component inspector, state timeline
//! and network panel. Click the counter and watch the timeline grow.
//!
//! Serve with `trunk serve` (or `ferrum dev`) and open the page.

use ferrum_core::{component, rsx, use_state};
use ferrum_devtools::{install, overlay};
use ferrum_web::AppHandle;
use wasm_bindgen::prelude::*;

#[component]
fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        section {
            h1 { "DevTools playground" }
            p { "Click the button; the overlay shows the state timeline." }
            button { onclick: move |_| set.update(|c| c + 1), "Count: {count}" }
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    // Install before the first render so the initial mount is captured.
    install();
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let app = AppHandle::new(root);
    app.render(|| {
        rsx! {
            div {
                Counter {}
                <> {overlay()} </>
            }
        }
    });
    // The overlay is a snapshot view; re-render once so the initial mount is
    // visible in the inspector.
    app.re_render();
    app.forget();
}
