//! # ferrum-native
//!
//! A native renderer backend for Ferrum. It implements the same
//! [`Renderer`] contract as `ferrum-web` and `ferrum-testing`, so components,
//! hooks, keyed reconciliation and context behave identically. The backend
//! stores a scene graph (elements, text, attributes, listeners) and can emit:
//!
//! - a serialized tree for debugging / snapshot tests,
//! - an **accessibility tree** (role + accessible name + children), the
//!   contract a wgpu/winit platform adapter consumes.
//!
//! This crate is deliberately dependency-free: the wgpu/winit window and
//! AccessKit bridge are platform adapters over this scene graph, not part of
//! the core renderer.

use ferrum_core::hooks::EffectQueue;
use ferrum_core::{AttrValue, FerrumEvent, VNode};
use ferrum_reconciler::{HydrationStats, ListenerHandle, NodeType, Renderer, TreeRenderer};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

// ---------------------------------------------------------------------------
// Scene graph
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct NativeNode(Rc<NativeNodeInner>);

struct NativeNodeInner {
    kind: RefCell<NativeKind>,
    parent: RefCell<Option<Weak<NativeNodeInner>>>,
    children: RefCell<Vec<NativeNode>>,
}

enum NativeKind {
    Element {
        tag: String,
        attrs: Vec<(String, AttrValue)>,
        listeners: Vec<(u64, String, ferrum_core::EventHandler)>,
    },
    Text(String),
}

impl NativeNode {
    fn element(tag: &str) -> Self {
        NativeNode(Rc::new(NativeNodeInner {
            kind: RefCell::new(NativeKind::Element {
                tag: tag.to_string(),
                attrs: Vec::new(),
                listeners: Vec::new(),
            }),
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
        }))
    }

    fn new_text(text: &str) -> Self {
        NativeNode(Rc::new(NativeNodeInner {
            kind: RefCell::new(NativeKind::Text(text.to_string())),
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
        }))
    }

    pub fn tag(&self) -> Option<String> {
        match &*self.0.kind.borrow() {
            NativeKind::Element { tag, .. } => Some(tag.clone()),
            NativeKind::Text(_) => None,
        }
    }

    pub fn text(&self) -> Option<String> {
        match &*self.0.kind.borrow() {
            NativeKind::Text(t) => Some(t.clone()),
            NativeKind::Element { .. } => None,
        }
    }

    pub fn attr(&self, name: &str) -> Option<AttrValue> {
        match &*self.0.kind.borrow() {
            NativeKind::Element { attrs, .. } => attrs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone()),
            NativeKind::Text(_) => None,
        }
    }

    pub fn children(&self) -> Vec<NativeNode> {
        self.0.children.borrow().clone()
    }

    pub fn parent(&self) -> Option<NativeNode> {
        self.0.parent.borrow().as_ref().and_then(|w| w.upgrade()).map(NativeNode)
    }

    fn set_parent(&self, parent: Option<&NativeNode>) {
        *self.0.parent.borrow_mut() = parent.map(|p| Rc::downgrade(&p.0));
    }

    fn detach(&self) {
        if let Some(parent) = self.parent() {
            parent
                .0
                .children
                .borrow_mut()
                .retain(|c| !NativeBackend::node_eq(c, self));
        }
        self.set_parent(None);
    }
}

// ---------------------------------------------------------------------------
// Backend
// ---------------------------------------------------------------------------

pub struct NativeBackend;

impl Renderer for NativeBackend {
    type Node = NativeNode;

    fn create_element(&self, tag: &str) -> NativeNode {
        NativeNode::element(tag)
    }

    fn create_text(&self, text: &str) -> NativeNode {
        NativeNode::new_text(text)
    }

    fn set_attribute(&self, node: &NativeNode, name: &str, value: &str) {
        if let NativeKind::Element { attrs, .. } = &mut *node.0.kind.borrow_mut() {
            set_attr(attrs, name, AttrValue::Str(value.to_string()));
        }
    }

    fn set_property(&self, node: &NativeNode, name: &str, value: &AttrValue) {
        if let NativeKind::Element { attrs, .. } = &mut *node.0.kind.borrow_mut() {
            set_attr(attrs, name, value.clone());
        }
    }

    fn remove_attribute(&self, node: &NativeNode, name: &str) {
        if let NativeKind::Element { attrs, .. } = &mut *node.0.kind.borrow_mut() {
            attrs.retain(|(n, _)| n != name);
        }
    }

    fn set_text(&self, node: &NativeNode, text: &str) {
        if let NativeKind::Text(t) = &mut *node.0.kind.borrow_mut() {
            *t = text.to_string();
        }
    }

    fn append_child(&self, parent: &NativeNode, child: &NativeNode) {
        child.detach();
        parent.0.children.borrow_mut().push(child.clone());
        child.set_parent(Some(parent));
    }

    fn insert_before(&self, parent: &NativeNode, child: &NativeNode, reference: Option<&NativeNode>) {
        child.detach();
        let mut children = parent.0.children.borrow_mut();
        let index = reference
            .and_then(|r| children.iter().position(|c| NativeBackend::node_eq(c, r)))
            .unwrap_or(children.len());
        children.insert(index, child.clone());
        drop(children);
        child.set_parent(Some(parent));
    }

    fn remove_child(&self, parent: &NativeNode, child: &NativeNode) {
        parent
            .0
            .children
            .borrow_mut()
            .retain(|c| !NativeBackend::node_eq(c, child));
        child.set_parent(None);
    }

    fn next_sibling(&self, node: &NativeNode) -> Option<NativeNode> {
        let parent = node.parent()?;
        let children = parent.0.children.borrow();
        let pos = children.iter().position(|c| NativeBackend::node_eq(c, node))?;
        children.get(pos + 1).cloned()
    }

    fn node_eq(a: &NativeNode, b: &NativeNode) -> bool {
        Rc::ptr_eq(&a.0, &b.0)
    }

    fn node_type(&self, node: &NativeNode) -> NodeType {
        match &*node.0.kind.borrow() {
            NativeKind::Element { tag, .. } => NodeType::Element(tag.clone()),
            NativeKind::Text(t) => NodeType::Text(t.clone()),
        }
    }

    fn first_child(&self, node: &NativeNode) -> Option<NativeNode> {
        node.0.children.borrow().first().cloned()
    }

    fn replace_child(&self, parent: &NativeNode, old: &NativeNode, new: &NativeNode) {
        let mut children = parent.0.children.borrow_mut();
        if let Some(pos) = children.iter().position(|c| NativeBackend::node_eq(c, old)) {
            children[pos] = new.clone();
        } else {
            children.push(new.clone());
        }
        drop(children);
        new.set_parent(Some(parent));
        old.set_parent(None);
    }

    fn add_listener(&self, node: &NativeNode, name: &str, handler: ferrum_core::EventHandler) -> ListenerHandle {
        let id = next_listener_id();
        if let NativeKind::Element { listeners, .. } = &mut *node.0.kind.borrow_mut() {
            listeners.push((id, name.to_string(), handler));
        }
        ListenerHandle(Box::new(id))
    }

    fn remove_listener(&self, node: &NativeNode, handle: ListenerHandle) {
        let id = *handle.0.downcast_ref::<u64>().expect("listener handle type");
        if let NativeKind::Element { listeners, .. } = &mut *node.0.kind.borrow_mut() {
            listeners.retain(|(i, _, _)| *i != id);
        }
    }
}

fn set_attr(attrs: &mut Vec<(String, AttrValue)>, name: &str, value: AttrValue) {
    match attrs.iter_mut().find(|(n, _)| n == name) {
        Some(slot) => slot.1 = value,
        None => attrs.push((name.to_string(), value)),
    }
}

fn next_listener_id() -> u64 {
    use std::cell::Cell;
    thread_local! {
        static NEXT: Cell<u64> = const { Cell::new(1) };
    }
    NEXT.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v
    })
}

// ---------------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------------

struct NativeInner {
    tree: TreeRenderer<NativeBackend>,
    render_fn: Option<Box<dyn FnMut() -> VNode>>,
    depth: usize,
}

/// A native host: render loop + effect flushing + event firing + queries.
#[derive(Clone)]
pub struct NativeApp {
    inner: Rc<RefCell<NativeInner>>,
    root: NativeNode,
}

impl NativeApp {
    pub fn new() -> Self {
        Self::with_root(NativeNode::element("#root"))
    }

    pub fn with_root(root: NativeNode) -> Self {
        let effects: EffectQueue = Rc::new(RefCell::new(Vec::new()));
        let tree = TreeRenderer::new(NativeBackend, root.clone(), effects);
        let app = NativeApp {
            inner: Rc::new(RefCell::new(NativeInner {
                tree,
                render_fn: None,
                depth: 0,
            })),
            root,
        };
        let weak = Rc::downgrade(&app.inner);
        let rr: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(inner) = weak.upgrade() {
                NativeApp::rerender(&inner);
            }
        });
        app.inner.borrow_mut().tree.set_re_render(rr);
        app
    }

    pub fn render(&self, mut f: impl FnMut() -> VNode + 'static) {
        {
            let mut inner = self.inner.borrow_mut();
            let vnode = f();
            inner.render_fn = Some(Box::new(f));
            inner.tree.render(&vnode);
        }
        let effects = self.inner.borrow().tree.effects().clone();
        ferrum_core::hooks::flush_effects(&effects);
    }

    pub fn hydrate(&self, mut f: impl FnMut() -> VNode + 'static) -> HydrationStats {
        let stats = {
            let mut inner = self.inner.borrow_mut();
            let vnode = f();
            inner.render_fn = Some(Box::new(f));
            inner.tree.hydrate(&vnode)
        };
        let effects = self.inner.borrow().tree.effects().clone();
        ferrum_core::hooks::flush_effects(&effects);
        stats
    }

    pub fn re_render(&self) {
        NativeApp::rerender(&self.inner);
    }

    fn rerender(inner: &Rc<RefCell<NativeInner>>) {
        {
            let mut i = inner.borrow_mut();
            if i.depth >= 50 {
                panic!("Ferrum: maximum update depth exceeded (50 nested renders)");
            }
            i.depth += 1;
            let NativeInner { tree, render_fn, .. } = &mut *i;
            if let Some(f) = render_fn.as_mut() {
                let vnode = f();
                tree.render(&vnode);
            }
        }
        let effects = inner.borrow().tree.effects().clone();
        ferrum_core::hooks::flush_effects(&effects);
        inner.borrow_mut().depth -= 1;
    }

    pub fn root(&self) -> NativeNode {
        self.root.clone()
    }

    pub fn find_all(&self, tag: &str) -> Vec<NativeNode> {
        let mut out = Vec::new();
        find_all_rec(&self.root, tag, &mut out);
        out
    }

    pub fn text(&self) -> String {
        text_content(&self.root)
    }

    pub fn html(&self) -> String {
        let mut out = String::new();
        html_rec(&self.root, &mut out);
        out
    }

    /// Fire the `nth` listener named `event` (0-based).
    pub fn fire(&self, event: &str, nth: usize) {
        self.fire_with(event, nth, Box::new(()));
    }

    pub fn fire_with(&self, event: &str, nth: usize, payload: Box<dyn Any>) {
        let handlers = self.collect_listeners(event);
        let (_, handler) = handlers
            .into_iter()
            .nth(nth)
            .unwrap_or_else(|| panic!("no listener `{}` #{}", event, nth));
        handler(&FerrumEvent::new(event, payload));
    }

    fn collect_listeners(&self, event: &str) -> Vec<(NativeNode, ferrum_core::EventHandler)> {
        let mut out = Vec::new();
        collect_listeners_rec(&self.root, event, &mut out);
        out
    }

    /// Build the accessibility tree (role + accessible name + children).
    pub fn a11y_tree(&self) -> A11yNode {
        let mut children = Vec::new();
        for child in self.root.children() {
            if let Some(node) = a11y_node(&child) {
                children.push(node);
            }
        }
        A11yNode {
            role: "root".to_string(),
            name: String::new(),
            children,
        }
    }
}

impl Default for NativeApp {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_listeners_rec(node: &NativeNode, event: &str, out: &mut Vec<(NativeNode, ferrum_core::EventHandler)>) {
    if let NativeKind::Element { listeners, .. } = &*node.0.kind.borrow() {
        for (_, name, handler) in listeners.iter() {
            if name == event || name.strip_prefix("on") == Some(event) {
                out.push((node.clone(), handler.clone()));
            }
        }
    }
    for child in node.children() {
        collect_listeners_rec(&child, event, out);
    }
}

fn find_all_rec(node: &NativeNode, tag: &str, out: &mut Vec<NativeNode>) {
    if node.tag().as_deref() == Some(tag) {
        out.push(node.clone());
    }
    for child in node.children() {
        find_all_rec(&child, tag, out);
    }
}

fn text_content(node: &NativeNode) -> String {
    if let Some(text) = node.text() {
        return text;
    }
    let mut out = String::new();
    for child in node.children() {
        out.push_str(&text_content(&child));
    }
    out
}

fn html_rec(node: &NativeNode, out: &mut String) {
    if let Some(text) = node.text() {
        out.push_str(&escape(&text));
        return;
    }
    let tag = node.tag().unwrap_or_default();
    out.push('<');
    out.push_str(&tag);
    if let NativeKind::Element { attrs, .. } = &*node.0.kind.borrow() {
        for (name, value) in attrs {
            match value {
                AttrValue::Bool(true) => out.push_str(&format!(" {}", name)),
                AttrValue::Bool(false) => {}
                _ => out.push_str(&format!(" {}=\"{}\"", name, escape(&value.to_string()))),
            }
        }
    }
    out.push('>');
    for child in node.children() {
        html_rec(&child, out);
    }
    out.push_str("</");
    out.push_str(&tag);
    out.push('>');
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Accessibility tree
// ---------------------------------------------------------------------------

/// A node in the native accessibility tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A11yNode {
    pub role: String,
    pub name: String,
    pub children: Vec<A11yNode>,
}

fn a11y_node(node: &NativeNode) -> Option<A11yNode> {
    let tag = node.tag()?;
    let role = role_for(&tag, node);
    let name = accessible_name(node, &tag, role);
    let mut children = Vec::new();
    for child in node.children() {
        if let Some(child) = a11y_node(&child) {
            children.push(child);
        }
    }
    Some(A11yNode {
        role: role.to_string(),
        name,
        children,
    })
}

fn role_for(tag: &str, node: &NativeNode) -> &'static str {
    match tag {
        "button" => "button",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "p" => "paragraph",
        "ul" | "ol" => "list",
        "li" => "listitem",
        "a" => "link",
        "nav" => "navigation",
        "main" => "main",
        "img" => "image",
        "input" => match node.attr("type").map(|v| v.to_string()).as_deref() {
            Some("checkbox") => "checkbox",
            Some("radio") => "radio",
            _ => "textbox",
        },
        _ => "generic",
    }
}

fn accessible_name(node: &NativeNode, tag: &str, role: &str) -> String {
    if let Some(label) = node.attr("aria-label") {
        return label.to_string();
    }
    // Generic containers are structural; their text is exposed by children.
    if role == "generic" {
        return String::new();
    }
    if tag == "img" {
        if let Some(alt) = node.attr("alt") {
            return alt.to_string();
        }
    }
    let text = text_content(node).trim().to_string();
    if !text.is_empty() {
        return text;
    }
    if let Some(value) = node.attr("value") {
        return value.to_string();
    }
    String::new()
}