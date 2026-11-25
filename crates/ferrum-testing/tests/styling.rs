//! Phase 5 gate: scoped CSS, themes, RTL + identical web/native rendering.

use ferrum_components::{Badge, Button, Card, ThemeRoot};
use ferrum_core::{component, rsx, VNode};
use ferrum_styling::{css, reset_registry, stylesheet, tw, Theme};
use ferrum_testing::TestRenderer;

/// Build `<ThemeRoot theme=..>child</ThemeRoot>` as a VNode.
fn themed(theme: Theme, child: VNode) -> VNode {
    VNode::component(
        "ThemeRoot",
        None,
        ThemeRoot {
            theme,
            children: vec![child],
        },
    )
}

#[test]
fn css_scoped_class_is_deterministic() {
    reset_registry();
    let a = css!("color: red;");
    let b = css!("color: red;");
    let c = css!("color: blue;");
    assert_eq!(a.name(), b.name(), "same rule -> same class");
    assert_ne!(a.name(), c.name(), "different rule -> different class");
    assert!(a.name().starts_with('f'));
}

#[test]
fn stylesheet_contains_registered_rules() {
    reset_registry();
    let _ = css! { color: red; padding: 4px; };
    let sheet = stylesheet();
    assert!(sheet.contains("color: red;"), "rule present: {sheet}");
    assert!(sheet.contains("padding: 4px;"));
}

#[test]
fn button_renders_scoped_classes() {
    reset_registry();
    let r = TestRenderer::new();
    r.render(|| themed(Theme::light(), rsx! { Button { variant: "primary", on_click: None, "Click" } }));
    let html = r.html();
    assert!(html.contains("<button"), "{html}");
    assert!(html.contains("class=\"f"), "scoped class missing: {html}");
    assert!(html.contains("Click"));
}

#[test]
fn theme_root_sets_tokens_and_dir() {
    reset_registry();
    let r = TestRenderer::new();
    r.render(|| themed(Theme::dark(), rsx! { Badge { label: "new" } }));
    let html = r.html();
    assert!(html.contains("--bg: #111827"), "{html}");
    assert!(html.contains("dir=\"ltr\""), "{html}");
}

#[test]
fn rtl_theme_sets_dir_rtl() {
    reset_registry();
    let r = TestRenderer::new();
    r.render(|| themed(Theme::dark().rtl(), rsx! { Badge { label: "x" } }));
    assert!(r.html().contains("dir=\"rtl\""));
}

#[component]
fn ThemedCard() -> VNode {
    // use_theme requires a provider; ThemeRoot supplies it.
    let theme = ferrum_styling::use_theme();
    let bg = theme.bg.clone();
    rsx! {
        Card { title: "Themed",
            span { style: "background: {bg}", "body" }
        }
    }
}

#[test]
fn theme_switch_rerenders_tokens() {
    reset_registry();
    let dark = std::rc::Rc::new(std::cell::Cell::new(true));
    let r = TestRenderer::new();
    let d2 = dark.clone();
    r.render(move || {
        let theme = if d2.get() { Theme::dark() } else { Theme::light() };
        themed(theme, rsx! { ThemedCard {} })
    });
    assert!(r.html().contains("--bg: #111827"));
    dark.set(false);
    r.re_render();
    assert!(r.html().contains("--bg: #ffffff"), "theme switched without remount");
}

#[test]
fn card_and_badge_render() {
    reset_registry();
    let r = TestRenderer::new();
    r.render(|| themed(Theme::light(), rsx! { Card { title: "T", Badge { label: "b" } } }));
    let html = r.html();
    assert!(html.contains("<h2"));
    assert!(html.contains("T"));
}

/// The shape the component library must produce (asserted identically by
/// the browser test in ferrum-web).
#[test]
fn component_library_snapshot() {
    reset_registry();
    let r = TestRenderer::new();
    r.render(|| themed(Theme::light(), rsx! { Button { variant: "ghost", on_click: None, "Hi" } }));
    let html = r.html();
    assert!(html.contains("<div style=\"--bg: #ffffff"), "{html}");
    assert!(html.contains("dir=\"ltr\""));
    assert!(html.contains("</button></div>"), "{html}");
}
#[test]
fn tailwind_utilities_compose_with_scoped_classes() {
    reset_registry();
    let scoped = css!("color: red;");
    let class = tw!("flex", "items-center", "gap-2");
    assert_eq!(class, "flex items-center gap-2");
    // A Tailwind class string and a scoped class can share one attribute.
    let combined = format!("{class} {}", scoped.name());
    assert!(combined.contains("flex"));
    assert!(combined.contains(scoped.name()));
}
