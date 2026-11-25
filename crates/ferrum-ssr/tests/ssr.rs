//! Phase 7 gate: SSR rendering (components/hooks/context/escaping/void/stream).

use ferrum_core::{component, rsx, use_context, use_state, Context};
use ferrum_ssr::{render_chunks, render_document, render_stream, render_to_string};

#[test]
fn renders_elements_text_and_escapes() {
    let html = render_to_string(&rsx! {
        div { class: "a", "<script> & \"q\"" }
    });
    assert_eq!(
        html,
        "<div class=\"a\">&lt;script&gt; &amp; \"q\"</div>"
    );
}

#[test]
fn renders_void_elements_without_closing_tag() {
    assert_eq!(render_to_string(&rsx! { br {} }), "<br>");
    assert_eq!(
        render_to_string(&rsx! { input { value: "x", disabled: true } }),
        "<input value=\"x\" disabled>"
    );
}

#[test]
fn renders_components_and_hooks_without_effects() {
    #[component]
    fn Counter() -> VNode {
        let (count, _set) = use_state(|| 7i32);
        rsx! { span { class: "n", "count={count}" } }
    }

    let html = render_to_string(&rsx! { Counter {} });
    assert_eq!(html, "<span class=\"n\">count=7</span>");
}

#[test]
fn renders_context_providers() {
    thread_local! {
        static CTX: Context<String> = Context::new();
    }

    #[component]
    fn Reader() -> VNode {
        let value = use_context(&CTX.with(|c| c.clone()));
        rsx! { p { "{value}" } }
    }

    let tree = CTX.with(|c| {
        c.provider(
            "dark".to_string(),
            vec![rsx! { Reader {} }],
        )
    });
    assert_eq!(render_to_string(&tree), "<p>dark</p>");
}

#[test]
fn renders_fragments_transparently() {
    let html = render_to_string(&rsx! {
        div { "a" <> "b" "c" </> "d" }
    });
    assert_eq!(html, "<div>abcd</div>");
}

#[test]
fn document_includes_title_and_scoped_styles() {
    let page = render_document("Home", "<h1>Hi</h1>");
    assert!(page.starts_with("<!DOCTYPE html>"));
    assert!(page.contains("<title>Home</title>"));
    assert!(page.contains("<div id=\"app\"><h1>Hi</h1></div>"));
    assert!(page.contains("id=\"ferrum-styles\""));
}

#[test]
fn stream_writes_flushed_chunks() {
    let tree = ferrum_core::VNode::fragment(vec![
        rsx! { section { "one" } },
        rsx! { section { "two" } },
    ]);
    let chunks = render_chunks(&tree);
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].contains("one"));
    assert!(chunks[1].contains("two"));

    let mut buf = Vec::new();
    render_stream(&tree, &mut buf).unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), chunks.concat());
}