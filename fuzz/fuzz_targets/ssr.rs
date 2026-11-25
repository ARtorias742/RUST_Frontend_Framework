//! Fuzz the SSR renderer with attacker-controlled text and attribute values.
#![no_main]

use ferrum_core::{rsx, VNode};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let node: VNode = rsx! {
        div { class: "fuzz", data: "{text}", p { "{text}" } }
    };
    let html = ferrum_ssr::render_to_string(&node);
    // The renderer must never emit an executable script element.
    assert!(!html.to_ascii_lowercase().contains("<script"));
});