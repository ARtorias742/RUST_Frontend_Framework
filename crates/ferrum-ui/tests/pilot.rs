//! Phase 1 gate: the same source renders on headless and native.

use ferrum_core::{rsx, VNode};
use ferrum_native::NativeApp;
use ferrum_testing::TestRenderer;
use ferrum_ui::{Button, Dialog};

fn button_tree() -> VNode {
    rsx! { Button { variant: "primary", disabled: false, on_click: None, "Save" } }
}

#[test]
fn button_renders_on_headless_and_native() {
    let node = button_tree();

    let headless = TestRenderer::new();
    headless.render(move || node.clone());
    let headless_html = headless.html();
    assert!(headless_html.contains("<button"));
    assert!(headless_html.contains("Save"));

    let native = NativeApp::new();
    let node2 = button_tree();
    native.render(move || node2.clone());
    let native_html = native.html();
    assert!(native_html.contains("<button"));
    assert!(native_html.contains("Save"));

    // Same tag and text on both backends.
    assert_eq!(
        headless.find_all("button").len(),
        native.find_all("button").len()
    );
}

#[test]
fn dialog_is_empty_when_closed_and_accessible_when_open() {
    let closed = rsx! { Dialog { open: false, title: "Hi", on_close: None } };
    let headless = TestRenderer::new();
    headless.render(move || closed.clone());
    assert_eq!(headless.find_all("div").len(), 0);

    let open = rsx! { Dialog { open: true, title: "Hi", on_close: None, "Body" } };
    let headless = TestRenderer::new();
    headless.render(move || open.clone());
    let dialog = headless.find_all("div").into_iter().find(|n| {
        headless
            .attr(n, "role")
            .map(|v| v.to_string() == "dialog")
            .unwrap_or(false)
    });
    let dialog = dialog.expect("dialog element");
    assert_eq!(
        headless.attr(&dialog, "aria-modal").map(|v| v.to_string()),
        Some("true".to_string())
    );
    assert_eq!(
        headless.attr(&dialog, "aria-label"),
        Some(ferrum_core::AttrValue::Str("Hi".into()))
    );
    assert!(headless.text().contains("Body"));

    let native = NativeApp::new();
    let open2 = rsx! { Dialog { open: true, title: "Hi", on_close: None, "Body" } };
    native.render(move || open2.clone());
    assert!(native.html().contains("role=\"dialog\""));
}
