//! Visual/contract snapshots for the five hero components.
//!
//! These are headless “visual” baselines: exact tag/role/ARIA structure. Pixel
//! baselines live in `e2e/` (Playwright).

use ferrum_core::rsx;
use ferrum_testing::TestRenderer;
use ferrum_ui::{Button, Dialog, Menu, MenuItem, TabItem, Tabs, Toast, ToastLevel, Toaster};

fn html(node: ferrum_core::VNode) -> String {
    let r = TestRenderer::new();
    r.render(move || node.clone());
    r.html()
}

#[test]
fn button_snapshot() {
    let html =
        html(rsx! { Button { variant: "primary", disabled: false, on_click: None, "Save" } });
    assert!(html.contains("<button"));
    assert!(html.contains("Save"));
    assert!(!html.contains("disabled=\"true\""));
}

#[test]
fn button_disabled_snapshot() {
    let html = html(rsx! { Button { variant: "ghost", disabled: true, on_click: None, "No" } });
    assert!(html.contains("disabled=\"true\""));
    assert!(html.contains("No"));
}

#[test]
fn dialog_snapshot() {
    let html = html(rsx! { Dialog { open: true, title: "Confirm", on_close: None, "Body" } });
    assert!(html.contains("role=\"dialog\""));
    assert!(html.contains("aria-modal"));
    assert!(html.contains("aria-label=\"Confirm\""));
    assert!(html.contains("Body"));
}

#[test]
fn menu_snapshot() {
    let items = vec![
        MenuItem {
            label: "One".into(),
            disabled: false,
            on_select: None,
        },
        MenuItem {
            label: "Two".into(),
            disabled: true,
            on_select: None,
        },
    ];
    let html = html(rsx! {
        Menu { open: true, active: 0usize, items: items, button { "Open" } }
    });
    assert!(html.contains("role=\"menu\""));
    assert!(html.contains("role=\"menuitem\""));
    assert!(html.contains("tabindex=\"0\""));
    assert!(html.contains("tabindex=\"-1\""));
    assert!(html.contains("aria-disabled"));
}

#[test]
fn tabs_snapshot() {
    let items = vec![
        TabItem {
            label: "One".into(),
            content: rsx! { p { "first" } },
        },
        TabItem {
            label: "Two".into(),
            content: rsx! { p { "second" } },
        },
    ];
    let html = html(
        rsx! { Tabs { orientation: "horizontal", active: 0usize, on_change: None, items: items } },
    );
    assert!(html.contains("role=\"tablist\""));
    assert!(html.contains("role=\"tab\""));
    assert!(html.contains("role=\"tabpanel\""));
    assert!(html.contains("aria-selected"));
    assert!(html.contains("aria-controls"));
    assert!(html.contains("first"));
    assert!(!html.contains("second"));
}

#[test]
fn toast_snapshot() {
    let toasts = vec![
        Toast {
            id: 1,
            level: ToastLevel::Success,
            title: "Saved".into(),
            description: "ok".into(),
        },
        Toast {
            id: 2,
            level: ToastLevel::Error,
            title: "Failed".into(),
            description: "no".into(),
        },
    ];
    let html = html(rsx! { Toaster { toasts: toasts, on_dismiss: None } });
    assert!(html.contains("aria-live=\"polite\""));
    assert!(html.contains("role=\"status\""));
    assert!(html.contains("role=\"alert\""));
    assert!(html.contains("Saved"));
    assert!(html.contains("Failed"));
}
