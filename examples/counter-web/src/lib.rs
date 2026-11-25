//! Phase 2/8 web example: the shared counter component, mounted in the DOM.
//!
//! Serve with `trunk serve` (or any static server) after
//! `wasm-pack build`; the page must contain `<div id="app"></div>`.

use example_shared::Counter;
use ferrum_core::rsx;
use ferrum_web::AppHandle;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let root = document.get_element_by_id("app").expect("#app not found");

    let app = AppHandle::new(root);
    app.render(|| {
        rsx! {
            div { class: "app",
                h1 { "Ferrum Counter (web)" }
                Counter {}
            }
        }
    });
    app.forget();
}
