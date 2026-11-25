//! # ferrum-ssr
//!
//! Server-side rendering: turn a [`VNode`] tree into HTML without a DOM.
//!
//! Components, hooks (state/reducer/memo/ref/context/id), fragments and
//! context providers are evaluated exactly once. Effects are **not** run
//! (they are client-only); the SSR pass is pure and side-effect free.
//!
//! ```ignore
//! let html = ferrum_ssr::render_to_string(&rsx! { h1 { "Hello" } });
//! let page = ferrum_ssr::render_document("Home", &html);
//! ```
//!
//! For streaming, [`render_chunks`] splits a top-level fragment into chunks
//! and [`render_stream`] writes them with a flush per chunk (the transport
//! can forward each chunk as it arrives).

use ferrum_core::hooks::{render_with, EffectQueue, Instance};
use ferrum_core::{AttrValue, VNode};
use std::rc::Rc;

/// Render a VNode tree to an HTML string.
pub fn render_to_string(vnode: &VNode) -> String {
    let mut out = String::new();
    write_node(vnode, &mut out);
    out
}

/// Render a VNode tree to any `std::io::Write` (file, socket, response body).
pub fn render_to_writer<W: std::io::Write>(vnode: &VNode, out: &mut W) -> std::io::Result<()> {
    out.write_all(render_to_string(vnode).as_bytes())
}

/// Split a tree into streamable chunks. A top-level fragment becomes one
/// chunk per child; anything else is a single chunk.
pub fn render_chunks(vnode: &VNode) -> Vec<String> {
    match vnode {
        VNode::Fragment(children) => children.iter().map(render_to_string).collect(),
        _ => vec![render_to_string(vnode)],
    }
}

/// Write chunks to `out`, flushing after each one.
pub fn render_stream<W: std::io::Write>(vnode: &VNode, out: &mut W) -> std::io::Result<()> {
    for chunk in render_chunks(vnode) {
        out.write_all(chunk.as_bytes())?;
        out.flush()?;
    }
    Ok(())
}

/// Render a complete HTML document (doctype, head, title, scoped styles).
pub fn render_document(title: &str, body: &str) -> String {
    let styles = ferrum_styling::stylesheet();
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\" />\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\
         <title>{}</title>\n<style id=\"ferrum-styles\">{}</style>\n</head>\n\
         <body><div id=\"app\">{}</div></body>\n</html>\n",
        escape_text(title),
        styles,
        body
    )
}

/// Render a matched route (target + params + nested outlet).
pub fn render_route(m: &ferrum_router::RouteMatch) -> String {
    let vnode = (m.target).render(&m.params, m.outlet.clone());
    render_to_string(&vnode)
}

// ---------------------------------------------------------------------------
// Node writer
// ---------------------------------------------------------------------------

fn write_node<W: std::fmt::Write>(vnode: &VNode, out: &mut W) {
    match vnode {
        VNode::Element(e) => {
            let _ = write!(out, "<{}", e.tag);
            for (name, value) in &e.attrs {
                match value {
                    AttrValue::Bool(true) => {
                        let _ = write!(out, " {}", name);
                    }
                    AttrValue::Bool(false) => {}
                    _ => {
                        let _ = write!(
                            out,
                            " {}=\"{}\"",
                            name,
                            escape_attr(&value.to_string())
                        );
                    }
                }
            }
            let _ = out.write_char('>');
            if is_void(&e.tag) {
                return;
            }
            for child in &e.children {
                write_node(child, out);
            }
            let _ = write!(out, "</{}>", e.tag);
        }
        VNode::Text(text) => {
            let _ = out.write_str(&escape_text(text));
        }
        VNode::Component(c) => {
            // Fresh instance per SSR pass; effects are discarded.
            let instance = Instance::new(c.name);
            let effects: EffectQueue = Rc::new(std::cell::RefCell::new(Vec::new()));
            let rendered = render_with(
                &instance,
                Rc::new(|| {}),
                effects,
                || c.props.render(),
            );
            write_node(&rendered, out);
        }
        VNode::Fragment(children) => {
            for child in children {
                write_node(child, out);
            }
        }
        VNode::Provider(p) => {
            ferrum_core::hooks::push_provider(p.ctx_id, p.value.clone());
            for child in &p.children {
                write_node(child, out);
            }
            ferrum_core::hooks::pop_provider();
        }
        VNode::Empty => {}
    }
}

fn is_void(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}