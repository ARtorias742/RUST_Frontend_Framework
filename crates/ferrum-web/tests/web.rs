//! Phase 2 gate: DOM renderer tests (run in a browser via wasm-bindgen-test).
//!
//! These require a browser driver (chromedriver/geckodriver) and
//! `wasm-bindgen-test-runner`; they cannot run in CI-less sandboxes.

use ferrum_core::{component, rsx};
use ferrum_web::WebRenderer;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

fn fresh_root() -> web_sys::Element {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    document
        .create_element("div")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .into()
}

fn text_of(el: &web_sys::Element) -> String {
    el.text_content().unwrap_or_default()
}

fn as_element(node: &web_sys::Node) -> web_sys::Element {
    node.dyn_ref::<web_sys::Element>().unwrap().clone()
}

fn html_of(node: &web_sys::Node) -> String {
    as_element(node).inner_html()
}

#[component]
fn Greeting(name: String) -> VNode {
    rsx! { span { class: "greeting", "Hello, {name}" } }
}

#[wasm_bindgen_test]
fn mounts_element_tree() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { div { class: "app", "hi" } });
    assert_eq!(root.inner_html(), "<div class=\"app\">hi</div>");
}

#[wasm_bindgen_test]
fn mounts_component() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { Greeting { name: "Ferrum" } });
    assert_eq!(
        root.inner_html(),
        "<span class=\"greeting\">Hello, Ferrum</span>"
    );
}

#[wasm_bindgen_test]
fn updates_text_in_place() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { p { "one" } });
    let p = root.first_child().unwrap();
    r.render(&rsx! { p { "two" } });
    assert_eq!(root.inner_html(), "<p>two</p>");
    // Same DOM node reused (identity preserved).
    assert_eq!(root.first_child().unwrap(), p);
}

#[wasm_bindgen_test]
fn updates_attrs() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { div { class: "a", id: "x" } });
    r.render(&rsx! { div { class: "b" } });
    assert_eq!(root.inner_html(), "<div class=\"b\"></div>");
}

#[wasm_bindgen_test]
fn keyed_children_reorder_without_remount() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { ul { li { key: 1, "a" } li { key: 2, "b" } } });
    let ul = root.first_child().unwrap();
    let first_li = ul.first_child().unwrap();

    // Reorder: [a, b] -> [b, a]
    r.render(&rsx! { ul { li { key: 2, "b" } li { key: 1, "a" } } });
    assert_eq!(html_of(&ul), "<li>b</li><li>a</li>");
    // The original <li>a</li> node was moved, not recreated.
    let new_first = ul.first_child().unwrap();
    assert_ne!(new_first, first_li);
    let second = ul.last_child().unwrap();
    assert_eq!(second, first_li);
}

#[wasm_bindgen_test]
fn keyed_removal_unmounts() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { ul { li { key: 1, "a" } li { key: 2, "b" } li { key: 3, "c" } } });
    r.render(&rsx! { ul { li { key: 1, "a" } li { key: 3, "c" } } });
    let ul = root.first_child().unwrap();
    assert_eq!(html_of(&ul), "<li>a</li><li>c</li>");
}

#[wasm_bindgen_test]
fn fragment_children_flatten_into_parent() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { div { "x" <> "y" "z" </> "w" } });
    assert_eq!(root.inner_html(), "<div>xyzw</div>");
}

#[wasm_bindgen_test]
fn empty_renders_nothing() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { div { if false { "no" } } });
    assert_eq!(root.inner_html(), "<div></div>");
}

#[wasm_bindgen_test]
fn type_change_replaces_node() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { p { "text" } });
    r.render(&rsx! { span { "text" } });
    assert_eq!(root.inner_html(), "<span>text</span>");
}

#[wasm_bindgen_test]
fn controlled_input_value_property() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    r.render(&rsx! { input { value: "one" } });
    let input: web_sys::HtmlInputElement = root.first_child().unwrap().dyn_into().unwrap();
    assert_eq!(input.value(), "one");
    r.render(&rsx! { input { value: "two" } });
    assert_eq!(input.value(), "two");
}

#[wasm_bindgen_test]
fn event_handler_fires() {
    use std::cell::Cell;
    use std::rc::Rc;
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    let fired = Rc::new(Cell::new(0));
    let f = fired.clone();
    r.render(&rsx! { button { onclick: move |_| f.set(f.get() + 1), "go" } });
    let button: HtmlElement = root.first_child().unwrap().dyn_into().unwrap();
    button.click();
    assert_eq!(fired.get(), 1);
    // Re-render replaces listeners with fresh ones; old handler is gone.
    let f2 = fired.clone();
    r.render(&rsx! { button { onclick: move |_| f2.set(f2.get() + 10), "go" } });
    button.click();
    assert_eq!(fired.get(), 11);
}

#[wasm_bindgen_test]
fn if_else_switches_subtree() {
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    let show = true;
    r.render(&rsx! { div { if show { "yes" } else { "no" } } });
    assert_eq!(root.inner_html(), "<div>yes</div>");
    let show = false;
    r.render(&rsx! { div { if show { "yes" } else { "no" } } });
    assert_eq!(root.inner_html(), "<div>no</div>");
}

#[wasm_bindgen_test]
fn text_content_helper_smoke() {
    // Guards the test helpers themselves.
    let root = fresh_root();
    root.set_inner_html("<b>x</b>");
    assert_eq!(text_of(&root), "x");
}
#[wasm_bindgen_test]
fn component_library_renders_identically_to_native() {
    // Same widget + theme as the native snapshot test in
    // ferrum-testing/tests/styling.rs: the DOM shape must match.
    use ferrum_components::{Button, ThemeRoot};
    use ferrum_styling::{reset_registry, Theme};
    reset_registry();
    let root = fresh_root();
    let mut r = WebRenderer::new(root.clone());
    let child = ferrum_core::rsx! { Button { variant: "ghost", on_click: None, "Hi" } };
    let tree = ferrum_core::VNode::component(
        "ThemeRoot",
        None,
        ThemeRoot {
            theme: Theme::light(),
            children: vec![child],
        },
    );
    r.render(&tree);
    let html = root.inner_html();
    assert!(html.contains("<div style=\"--bg: #ffffff"), "{html}");
    assert!(html.contains("dir=\"ltr\""));
    assert!(html.contains("</button></div>"));
}

#[wasm_bindgen_test]
fn hydrates_server_html_and_fixes_text_in_place() {
    let root = fresh_root();
    root.set_inner_html("<div class=\"app\"><h1>Server</h1><p>one</p></div>");
    let h1 = root.first_child().unwrap().first_child().unwrap();

    let mut r = WebRenderer::new(root.clone());
    let stats = r.hydrate(&rsx! {
        div { class: "app",
            h1 { "Server" }
            p { "two" }
        }
    });

    assert_eq!(stats.text_fixed, 1, "{stats:?}");
    assert_eq!(stats.tag_replaced, 0, "{stats:?}");
    assert_eq!(
        root.inner_html(),
        "<div class=\"app\"><h1>Server</h1><p>two</p></div>"
    );
    // Server <h1> reused.
    assert_eq!(h1, root.first_child().unwrap().first_child().unwrap());
}

#[wasm_bindgen_test]
fn hydration_recovers_tag_mismatch() {
    let root = fresh_root();
    root.set_inner_html("<div><p>old</p></div>");
    let mut r = WebRenderer::new(root.clone());
    let stats = r.hydrate(&rsx! { div { span { "new" } } });
    assert_eq!(stats.tag_replaced, 1, "{stats:?}");
    assert_eq!(root.inner_html(), "<div><span>new</span></div>");
}
