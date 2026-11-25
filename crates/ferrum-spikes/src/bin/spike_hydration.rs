//! Phase 0.5 spike #2: hydration (server HTML string -> client attach) with
//! mismatch detection + recovery, on a toy DOM (native, no browser needed).
//!
//! Run: `cargo run -p ferrum-spikes --bin spike_hydration`
//!
//! Flow: build a VNode tree (the "app"), render it to an HTML string
//! (the "server"), parse that string back into a toy DOM (the "browser's
//! existing DOM"), then hydrate: walk DOM + VNode in parallel, attach where
//! they match, and on mismatch (text/tag) fall back to client re-render of
//! that subtree (recovery). Fragments are transparent (no DOM node), exactly
//! like a real hydrator. Measures hydration time and mismatch counts.

use ferrum_core::{rsx, VNode};
use std::time::Instant;

#[derive(Debug, Clone)]
enum TNode {
    Element {
        tag: String,
        #[allow(dead_code)]
        attrs: Vec<(String, String)>,
        children: Vec<TNode>,
    },
    Text(String),
}

// --- server: VNode -> HTML string -------------------------------------------

fn render_html(v: &VNode, out: &mut String) {
    match v {
        VNode::Element(e) => {
            out.push('<');
            out.push_str(&e.tag);
            for (k, val) in &e.attrs {
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                out.push_str(&val.to_string().replace('"', "&quot;"));
                out.push('"');
            }
            out.push('>');
            for c in &e.children {
                render_html(c, out);
            }
            out.push_str("</");
            out.push_str(&e.tag);
            out.push('>');
        }
        VNode::Text(t) => out.push_str(t),
        VNode::Component(c) => render_html(&ferrum_core::hooks::render_isolated(&c.props), out),
        VNode::Fragment(ch) => {
            for c in ch {
                render_html(c, out);
            }
        }
        VNode::Provider(p) => {
            for c in &p.children {
                render_html(c, out);
            }
        }
        VNode::Empty => {}
    }
}

// --- toy HTML parser (subset: well-formed HTML from render_html) -------------

fn parse_html(s: &str) -> TNode {
    let b = s.as_bytes();
    let mut i = 0usize;
    parse_node(b, &mut i)
}

fn parse_node(b: &[u8], i: &mut usize) -> TNode {
    skip_ws(b, i);
    if *i >= b.len() {
        return TNode::Text(String::new());
    }
    if b[*i] == b'<' {
        *i += 1; // consume '<'
        let tag_start = *i;
        while *i < b.len() && b[*i] != b'>' && b[*i] != b' ' {
            *i += 1;
        }
        let tag = String::from_utf8_lossy(&b[tag_start..*i]).to_string();
        let mut attrs = Vec::new();
        loop {
            skip_ws(b, i);
            if *i >= b.len() {
                break;
            }
            if b[*i] == b'>' {
                *i += 1;
                break;
            }
            if b[*i] == b'/' {
                *i += 2; // self-closing
                return TNode::Element {
                    tag,
                    attrs,
                    children: Vec::new(),
                };
            }
            let name_start = *i;
            while *i < b.len() && b[*i] != b'=' && b[*i] != b'>' && b[*i] != b' ' {
                *i += 1;
            }
            let name = String::from_utf8_lossy(&b[name_start..*i]).to_string();
            skip_ws(b, i);
            let mut value = String::new();
            if *i < b.len() && b[*i] == b'=' {
                *i += 1;
                skip_ws(b, i);
                if *i < b.len() && b[*i] == b'"' {
                    *i += 1;
                    let v_start = *i;
                    while *i < b.len() && b[*i] != b'"' {
                        *i += 1;
                    }
                    value = String::from_utf8_lossy(&b[v_start..*i]).to_string();
                    *i += 1;
                }
            }
            attrs.push((name, value));
        }
        let mut children = Vec::new();
        loop {
            if *i >= b.len() {
                break;
            }
            if b[*i] == b'<' && *i + 1 < b.len() && b[*i + 1] == b'/' {
                *i += 2;
                while *i < b.len() && b[*i] != b'>' {
                    *i += 1;
                }
                *i += 1;
                break;
            }
            children.push(parse_node(b, i));
        }
        TNode::Element {
            tag,
            attrs,
            children,
        }
    } else {
        let t_start = *i;
        while *i < b.len() && b[*i] != b'<' {
            *i += 1;
        }
        TNode::Text(String::from_utf8_lossy(&b[t_start..*i]).to_string())
    }
}

fn skip_ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && (b[*i] == b' ' || b[*i] == b'\n' || b[*i] == b'\t') {
        *i += 1;
    }
}

// --- client render (recovery) ------------------------------------------------

fn to_toy(v: &VNode) -> TNode {
    match v {
        VNode::Element(e) => TNode::Element {
            tag: e.tag.clone(),
            attrs: e
                .attrs
                .iter()
                .map(|(k, val)| (k.clone(), val.to_string()))
                .collect(),
            children: e.children.iter().map(to_toy).collect(),
        },
        VNode::Text(t) => TNode::Text(t.clone()),
        VNode::Component(c) => to_toy(&ferrum_core::hooks::render_isolated(&c.props)),
        VNode::Fragment(ch) => TNode::Element {
            tag: "#fragment".into(),
            attrs: vec![],
            children: ch.iter().map(to_toy).collect(),
        },
        VNode::Provider(p) => TNode::Element {
            tag: "#fragment".into(),
            attrs: vec![],
            children: p.children.iter().map(to_toy).collect(),
        },
        VNode::Empty => TNode::Text(String::new()),
    }
}

// --- hydration ---------------------------------------------------------------

#[derive(Default, Debug)]
struct Stats {
    matched: usize,
    text_fixed: usize,
    tag_replaced: usize,
    children_appended: usize,
    children_removed: usize,
}

/// Hydrate one DOM node against one VNode.
fn hydrate(dom: &mut TNode, vdom: &VNode, st: &mut Stats) {
    match vdom {
        // Fragments/providers have no DOM node: their children hydrate
        // against the current DOM parent's children, in order (transparent).
        VNode::Fragment(children) => {
            if let TNode::Element { children: dchildren, .. } = dom {
                hydrate_children(dchildren, children, st);
            } else {
                *dom = to_toy(vdom);
                st.tag_replaced += 1;
            }
        }
        VNode::Provider(p) => {
            if let TNode::Element { children: dchildren, .. } = dom {
                hydrate_children(dchildren, &p.children, st);
            } else {
                *dom = to_toy(vdom);
                st.tag_replaced += 1;
            }
        }
        VNode::Empty => {}
        _ => match dom {
            TNode::Element { tag, children, .. } => {
                if let VNode::Element(e) = vdom {
                    if tag == &e.tag {
                        st.matched += 1;
                        hydrate_children(children, &e.children, st);
                        return;
                    }
                }
                // Tag mismatch (or type mismatch): recovery = client re-render.
                *dom = to_toy(vdom);
                st.tag_replaced += 1;
            }
            TNode::Text(t) => {
                if let VNode::Text(v) = vdom {
                    if t != v {
                        if std::env::var("SPIKE_DEBUG").is_ok() {
                            println!("TEXT FIX: {:?} -> {:?}", t, v);
                        }
                        *t = v.clone();
                        st.text_fixed += 1;
                    } else {
                        st.matched += 1;
                    }
                } else {
                    *dom = to_toy(vdom);
                    st.tag_replaced += 1;
                }
            }
        },
    }
}

/// Hydrate a list of VNode children against a list of DOM children.
/// Unkeyed, position-based (keys are Phase 7 work); fragments are flattened
/// and share the same DOM index.
fn hydrate_children(dom: &mut Vec<TNode>, vdom: &[VNode], st: &mut Stats) {
    let mut di = 0usize;
    hydrate_children_at(dom, vdom, &mut di, st);
    if dom.len() > di {
        st.children_removed += dom.len() - di;
        dom.truncate(di);
    }
}

fn hydrate_children_at(dom: &mut Vec<TNode>, vdom: &[VNode], di: &mut usize, st: &mut Stats) {
    for v in vdom {
        match v {
            VNode::Fragment(ch) => hydrate_children_at(dom, ch, di, st),
            VNode::Empty => {}
            _ => {
                if *di < dom.len() {
                    hydrate(&mut dom[*di], v, st);
                    *di += 1;
                } else {
                    dom.push(to_toy(v));
                    st.children_appended += 1;
                    *di += 1;
                }
            }
        }
    }
}

fn main() {
    // The "app": a 300-row list (rows embedded as a fragment, like real apps).
    let rows: Vec<VNode> = (0..300).map(|i| rsx! { li { class: "row", "row {i}" } }).collect();
    let app = rsx! { ul { class: "list", {rows} } };

    // Server render + transport (string) + parse (browser's existing DOM).
    let mut html = String::new();
    render_html(&app, &mut html);
    let base = parse_html(&html);
    println!("== spike-hydration ==");
    println!("server HTML size:    {} bytes", html.len());

    // Client re-renders with a small change: 10 texts differ, 5 rows appended.
    let rows2: Vec<VNode> = (0..305)
        .map(|i| {
            if i < 10 {
                rsx! { li { class: "row", "row {i} CHANGED" } }
            } else {
                rsx! { li { class: "row", "row {i}" } }
            }
        })
        .collect();
    let app2 = rsx! { ul { class: "list", {rows2} } };

    // Measure hydration (fresh DOM clone each iteration).
    let iters = 100;
    let start = Instant::now();
    let mut last = Stats::default();
    for _ in 0..iters {
        let mut dom = base.clone();
        let mut st = Stats::default();
        hydrate(&mut dom, &app2, &mut st);
        last = st;
    }
    let elapsed = start.elapsed();
    let per_us = elapsed.as_secs_f64() * 1e6 / iters as f64;

    println!(
        "hydrated nodes:      {}",
        last.matched + last.text_fixed + last.tag_replaced
    );
    println!("matched:             {}", last.matched);
    println!("text mismatches:     {} (fixed in place)", last.text_fixed);
    println!(
        "tag replacements:    {} (client re-render recovery)",
        last.tag_replaced
    );
    println!("children appended:   {}", last.children_appended);
    println!("children removed:    {}", last.children_removed);
    println!("iterations:          {iters}");
    println!("total time:          {:.2} ms", elapsed.as_secs_f64() * 1e3);
    println!("per-hydration time:  {per_us:.2} us");
    println!("verdict:             hydration attaches in O(n); mismatches recover locally");
}