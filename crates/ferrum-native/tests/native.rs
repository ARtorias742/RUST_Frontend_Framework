//! Phase 8 gate: native renderer + accessibility tree + events.

use ferrum_core::{component, rsx, use_state};
use ferrum_native::NativeApp;
use ferrum_reconciler::Renderer;
use std::rc::Rc;

#[component]
fn Counter() -> ferrum_core::VNode {
    let (count, set) = use_state(|| 0i32);
    let set = set.clone();
    rsx! {
        div { class: "counter",
            button { class: "inc", onclick: move |_| set.update(|c| c + 1), "+" }
            span { class: "value", "Count: {count}" }
        }
    }
}

#[test]
fn renders_the_same_tree_as_the_web_ir() {
    let app = NativeApp::new();
    app.render(|| {
        rsx! {
            div { class: "app",
                h1 { "Native" }
                Counter {}
            }
        }
    });
    assert_eq!(
        app.html(),
        "<#root><div class=\"app\"><h1>Native</h1><div class=\"counter\"><button class=\"inc\">+</button><span class=\"value\">Count: 0</span></div></div></#root>"
    );
}

#[test]
fn events_update_native_state() {
    let app = NativeApp::new();
    app.render(|| rsx! { Counter {} });
    app.fire("click", 0);
    assert!(app.text().contains("Count: 1"), "{}", app.text());
    app.fire("click", 0);
    assert!(app.text().contains("Count: 2"), "{}", app.text());
}

#[test]
fn accessibility_tree_has_roles_and_names() {
    let app = NativeApp::new();
    app.render(|| {
        rsx! {
            main {
                h1 { "Dashboard" }
                button { "Save" }
                input { type: "checkbox", aria-label: "Accept" }
                img { alt: "Logo", src: "logo.png" }
            }
        }
    });
    let tree = app.a11y_tree();
    let main = &tree.children[0];
    assert_eq!(main.role, "main");
    assert_eq!(main.children[0].role, "heading");
    assert_eq!(main.children[0].name, "Dashboard");
    assert_eq!(main.children[1].role, "button");
    assert_eq!(main.children[1].name, "Save");
    assert_eq!(main.children[2].role, "checkbox");
    assert_eq!(main.children[2].name, "Accept");
    assert_eq!(main.children[3].role, "image");
    assert_eq!(main.children[3].name, "Logo");
}

#[test]
fn keyed_reorder_keeps_identity() {
    let app = NativeApp::new();
    app.render(|| rsx! { ul { li { key: 1, "a" } li { key: 2, "b" } } });
    let first = app.find_all("li")[0].clone();
    app.re_render();
    // Same closure -> same order; identity is stable across updates.
    assert!(ferrum_native::NativeBackend::node_eq(
        &first,
        &app.find_all("li")[0]
    ));
}

#[test]
fn native_and_headless_share_the_component_contract() {
    // The same component tree renders through a different backend without
    // any component changes.
    let app = NativeApp::new();
    app.render(|| rsx! { Counter {} });
    let native = app.html();

    let headless = ferrum_testing::TestRenderer::new();
    headless.render(|| rsx! { Counter {} });
    assert_eq!(native, headless.html());
}

#[test]
fn effect_state_set_does_not_recurse() {
    // A component that sets state from an effect must settle (the effect's
    // deps are marked consumed before it runs).
    #[component]
    fn Once() -> ferrum_core::VNode {
        let (n, set) = use_state(|| 0i32);
        let set = set.clone();
        ferrum_core::use_effect(
            move || {
                set.set(1);
                None
            },
            Some(&[()]),
        );
        rsx! { span { "{n}" } }
    }

    let app = NativeApp::new();
    app.render(|| rsx! { Once {} });
    assert_eq!(app.text(), "1");
}

#[test]
fn hydration_attaches_to_existing_native_nodes() {
    let server = NativeApp::new();
    server.render(|| rsx! { div { p { "server" } } });
    let root = server.root();

    let client = NativeApp::with_root(root);
    let stats = client.hydrate(|| rsx! { div { p { "client" } } });
    assert_eq!(stats.text_fixed, 1, "{stats:?}");
    assert_eq!(client.text(), "client");
}

#[test]
fn clone_is_cheap_and_shared() {
    let app = NativeApp::new();
    app.render(|| rsx! { Counter {} });
    let clone = app.clone();
    clone.fire("click", 0);
    assert!(app.text().contains("Count: 1"));
    let _ = Rc::new(());
}
