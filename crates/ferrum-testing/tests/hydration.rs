//! Phase 7 gate: hydration (server DOM -> client attach + mismatch recovery).

use ferrum_core::{component, rsx};
use ferrum_reconciler::Renderer;
use ferrum_testing::TestRenderer;
use std::cell::Cell;
use std::rc::Rc;

#[component]
fn App(label: String) -> VNode {
    rsx! {
        div { class: "app",
            h1 { "Server" }
            p { "{label}" }
        }
    }
}

#[test]
fn hydrates_existing_dom_and_fixes_text_in_place() {
    let server = TestRenderer::new();
    server.render(|| rsx! { App { label: "one" } });
    let root = server.root();
    let h1 = server.find_all("h1")[0].clone();

    let client = TestRenderer::with_root(root);
    let stats = client.hydrate(|| rsx! { App { label: "two" } });

    assert!(stats.matched > 0, "{stats:?}");
    assert_eq!(stats.text_fixed, 1, "{stats:?}");
    assert_eq!(stats.tag_replaced, 0, "{stats:?}");
    assert!(client.html().contains("two"));
    // The server <h1> node was reused, not recreated.
    assert!(ferrum_testing::HeadlessBackend::node_eq(
        &h1,
        &client.find_all("h1")[0]
    ));
}

#[test]
fn tag_mismatch_recovers_by_replacing_subtree() {
    let server = TestRenderer::new();
    server.render(|| rsx! { div { p { "old" } } });
    let root = server.root();

    let client = TestRenderer::with_root(root);
    let stats = client.hydrate(|| rsx! { div { span { "new" } } });

    assert_eq!(stats.tag_replaced, 1, "{stats:?}");
    assert_eq!(client.html(), "<#root><div><span>new</span></div></#root>");
}

#[test]
fn extra_server_children_are_removed() {
    let server = TestRenderer::new();
    server.render(|| rsx! { div { span { "a" } span { "b" } } });
    let root = server.root();

    let client = TestRenderer::with_root(root);
    let stats = client.hydrate(|| rsx! { div { span { "a" } } });

    assert_eq!(stats.children_removed, 1, "{stats:?}");
    assert_eq!(client.html(), "<#root><div><span>a</span></div></#root>");
}

#[test]
fn hydration_attaches_listeners() {
    let server = TestRenderer::new();
    server.render(|| rsx! { button { "go" } });
    let root = server.root();

    let fired = Rc::new(Cell::new(0));
    let f = fired.clone();
    let client = TestRenderer::with_root(root);
    client.hydrate(move || {
        let f = f.clone();
        rsx! { button { onclick: move |_| f.set(f.get() + 1), "go" } }
    });

    client.fire("click", 0);
    assert_eq!(fired.get(), 1);
}

#[test]
fn fragments_are_transparent_during_hydration() {
    let server = TestRenderer::new();
    server.render(|| rsx! { div { "a" <> "b" "c" </> "d" } });
    let root = server.root();

    let client = TestRenderer::with_root(root);
    let stats = client.hydrate(|| rsx! { div { "a" <> "B" "c" </> "d" } });

    assert_eq!(stats.text_fixed, 1, "{stats:?}");
    assert_eq!(client.text(), "aBcd");
}