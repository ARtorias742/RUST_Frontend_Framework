//! Phase 1 gate: `rsx!` + component IR + debug renderer, 20+ tests.

use ferrum_core::{component, render_debug, rsx, AttrValue, FerrumEvent, VNode};

#[component]
fn Counter(count: i32) -> VNode {
    rsx! {
        button { class: "counter", "Count: {count}" }
    }
}

#[component]
fn Card(title: String, children: Vec<VNode>) -> VNode {
    rsx! {
        div { class: "card",
            h2 { "{title}" }
            div { class: "body", {children} }
        }
    }
}

#[component]
fn NoChildren(label: String) -> VNode {
    rsx! { span { "{label}" } }
}

fn el(v: &VNode) -> &ferrum_core::VElement {
    match v {
        VNode::Element(e) => e,
        other => panic!("expected element, got {:?}", other),
    }
}

// --- elements ---------------------------------------------------------------

#[test]
fn element_basic() {
    let v = rsx! { div {} };
    assert_eq!(el(&v).tag, "div");
    assert!(el(&v).children.is_empty());
}

#[test]
fn element_with_attrs() {
    let v = rsx! { div { class: "app", id: "root", hidden: true } };
    let e = el(&v);
    assert_eq!(
        e.attrs[0],
        ("class".to_string(), AttrValue::Str("app".into()))
    );
    assert_eq!(
        e.attrs[1],
        ("id".to_string(), AttrValue::Str("root".into()))
    );
    assert_eq!(e.attrs[2], ("hidden".to_string(), AttrValue::Bool(true)));
}

#[test]
fn element_dashed_attr() {
    let v = rsx! { div { data-id: 7 } };
    assert_eq!(el(&v).attrs[0].0, "data-id");
    assert_eq!(el(&v).attrs[0].1, AttrValue::Int(7));
}

#[test]
fn text_child() {
    let v = rsx! { p { "hello" } };
    match &el(&v).children[0] {
        VNode::Text(t) => assert_eq!(t, "hello"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn text_interpolation() {
    let name = "Ferrum";
    let v = rsx! { p { "Hello, {name}!" } };
    match &el(&v).children[0] {
        VNode::Text(t) => assert_eq!(t, "Hello, Ferrum!"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn text_escaped_braces() {
    let v = rsx! { p { "{{not interp}}" } };
    match &el(&v).children[0] {
        VNode::Text(t) => assert_eq!(t, "{not interp}"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn number_text_child() {
    let v = rsx! { p { 42 } };
    match &el(&v).children[0] {
        VNode::Text(t) => assert_eq!(t, "42"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn nested_elements() {
    let v = rsx! { div { span { "a" } } };
    let inner = el(&el(&v).children[0]);
    assert_eq!(inner.tag, "span");
}

#[test]
fn empty_element_renders_in_debug() {
    let v = rsx! { div {} };
    assert_eq!(render_debug(&v), "<div>\n</div>\n");
}

// --- components ---------------------------------------------------------------

#[test]
fn component_basic() {
    let v = rsx! { Counter { count: 5 } };
    match &v {
        VNode::Component(c) => assert_eq!(c.name, "Counter"),
        other => panic!("expected component, got {:?}", other),
    }
}

#[test]
fn component_renders() {
    let v = rsx! { Counter { count: 5 } };
    let out = render_debug(&v);
    assert!(out.contains("<Counter>"));
    assert!(out.contains("Count: 5"));
}

#[test]
fn component_prop_into_string() {
    // &str prop coerces into String field via `.into()`
    let v = rsx! { NoChildren { label: "hi" } };
    assert!(render_debug(&v).contains("hi"));
}

#[test]
fn component_with_children_block() {
    let v = rsx! {
        Card { title: "Hello" } {
            "body text"
        }
    };
    let out = render_debug(&v);
    assert!(out.contains("Hello"));
    assert!(out.contains("body text"));
}

#[test]
fn component_without_children_block() {
    let v = rsx! { NoChildren { label: "x" } };
    // children field defaults to empty vec
    let out = render_debug(&v);
    assert!(out.contains("x"));
}

#[test]
fn component_with_key() {
    let v = rsx! { Counter { key: 3, count: 1 } };
    match &v {
        VNode::Component(c) => assert_eq!(c.key, Some(ferrum_core::Key(3))),
        other => panic!("expected component, got {:?}", other),
    }
}

// --- control flow -------------------------------------------------------------

#[test]
fn if_expr() {
    let show = true;
    let v = rsx! { if show { "yes" } };
    match &v {
        VNode::Text(t) => assert_eq!(t, "yes"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn if_else_expr() {
    let show = false;
    let v = rsx! { if show { "yes" } else { "no" } };
    match &v {
        VNode::Text(t) => assert_eq!(t, "no"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn if_without_else_is_empty() {
    let show = false;
    let v = rsx! { if show { "yes" } };
    assert!(matches!(v, VNode::Empty));
}

#[test]
fn for_loop() {
    let v = rsx! { for i in 0..3 { li { "{i}" } } };
    let frag = match &v {
        VNode::Fragment(f) => f,
        other => panic!("expected fragment, got {:?}", other),
    };
    assert_eq!(frag.len(), 3);
    assert!(render_debug(&v).contains("0"));
    assert!(render_debug(&v).contains("2"));
}

#[test]
fn for_loop_with_key() {
    let items = vec![("a", 1), ("b", 2)];
    let v = rsx! {
        for (name, id) in items { li { key: id, "{name}" } }
    };
    let out = render_debug(&v);
    assert!(out.contains("a"));
    assert!(out.contains("b"));
}

#[test]
fn if_inside_element() {
    let done = true;
    let v = rsx! { ul { if done { li { "done" } } } };
    assert!(render_debug(&v).contains("done"));
}

// --- fragments & embedded exprs ------------------------------------------------

#[test]
fn fragment_multiple_roots() {
    let v = rsx! { h1 { "A" } p { "B" } };
    match &v {
        VNode::Fragment(f) => assert_eq!(f.len(), 2),
        other => panic!("expected fragment, got {:?}", other),
    }
}

#[test]
fn explicit_fragment_syntax() {
    let v = rsx! { <> h1 { "A" } p { "B" } </> };
    match &v {
        VNode::Fragment(f) => assert_eq!(f.len(), 2),
        other => panic!("expected fragment, got {:?}", other),
    }
}

#[test]
fn embedded_expression_child() {
    let extra = rsx! { em { "extra" } };
    let v = rsx! { p { "a" {extra} "b" } };
    let out = render_debug(&v);
    assert!(out.contains("extra"));
}

// --- events ---------------------------------------------------------------------

#[test]
fn event_listener_stored() {
    let v = rsx! { button { onclick: move |_e: &FerrumEvent| {}, "go" } };
    let e = el(&v);
    assert_eq!(e.listeners.len(), 1);
    assert_eq!(e.listeners[0].0, "onclick");
}

// --- debug renderer ---------------------------------------------------------------

#[test]
fn debug_render_full_tree() {
    let v = rsx! {
        div { class: "app",
            h1 { "Title" }
            Counter { count: 2 }
        }
    };
    let out = render_debug(&v);
    let expected = "\
<div class=\"app\">
  <h1>
    text: \"Title\"
  </h1>
  <Counter>
    <button class=\"counter\">
      text: \"Count: 2\"
    </button>
  </Counter>
</div>
";
    assert_eq!(out, expected);
}
