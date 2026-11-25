//! `#[component(default)]` generates `Default` for the props struct.

use ferrum_core::{component, rsx, ComponentProps};

#[component(default)]
fn Defaults(label: String, count: i32) -> VNode {
    rsx! { span { "{label}:{count}" } }
}

#[test]
fn default_props_are_generated() {
    let props = Defaults::default();
    assert_eq!(props.label, "");
    assert_eq!(props.count, 0);
    assert_eq!(props.children.len(), 0);
}

#[test]
fn default_props_render() {
    let node = Defaults::default().render();
    let html = ferrum_core::render_debug(&node);
    assert!(html.contains("span"));
    assert!(html.contains(":0"));
}
