use crate::vnode::{AttrValue, VNode};

/// Render a [`VNode`] tree to an indented debug string (Phase 1 gate).
///
/// Components are expanded (their `render()` output is inlined) and shown as
/// `<Name>` ... `</Name>` so component boundaries stay visible.
pub fn render_debug(v: &VNode) -> String {
    let mut out = String::new();
    render_node(v, 0, &mut out);
    out
}

fn render_node(v: &VNode, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match v {
        VNode::Element(e) => {
            out.push_str(&pad);
            out.push('<');
            out.push_str(&e.tag);
            for (name, value) in &e.attrs {
                out.push(' ');
                out.push_str(name);
                out.push_str("=\"");
                out.push_str(&escape_attr(&value.to_string()));
                out.push('"');
            }
            if e.listeners.is_empty() {
                out.push_str(">\n");
            } else {
                out.push_str(" [");
                for (i, (name, _)) in e.listeners.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(name);
                }
                out.push_str("]>\n");
            }
            for c in &e.children {
                render_node(c, indent + 1, out);
            }
            out.push_str(&pad);
            out.push_str("</");
            out.push_str(&e.tag);
            out.push_str(">\n");
        }
        VNode::Text(t) => {
            out.push_str(&pad);
            out.push_str("text: ");
            out.push_str(&format!("{:?}", t));
            out.push('\n');
        }
        VNode::Component(c) => {
            out.push_str(&pad);
            out.push('<');
            out.push_str(c.name);
            out.push_str(">\n");
            let rendered = crate::hooks::render_isolated(&c.props);
            render_node(&rendered, indent + 1, out);
            out.push_str(&pad);
            out.push_str("</");
            out.push_str(c.name);
            out.push_str(">\n");
        }
        VNode::Fragment(children) => {
            for c in children {
                render_node(c, indent, out);
            }
        }
        VNode::Provider(p) => {
            for c in &p.children {
                render_node(c, indent, out);
            }
        }
        VNode::Empty => {}
    }
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;")
}

// Keep AttrValue linked into this module's docs.
#[allow(dead_code)]
fn _assert_attr(_: AttrValue) {}