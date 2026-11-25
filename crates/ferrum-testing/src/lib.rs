//! # ferrum-testing
//!
//! A headless DOM ([`HeadlessBackend`]) + [`TestRenderer`] so Ferrum apps
//! (hooks, router, data) can be tested natively, without a browser.
//!
//! The renderer shares the production reconciler
//! ([`ferrum_reconciler::TreeRenderer`]), so component instances, hooks,
//! effects, keyed diffing and context behave exactly as in the browser.

use ferrum_core::hooks::EffectQueue;
use ferrum_core::{AttrValue, FerrumEvent, VNode};
use ferrum_reconciler::{HydrationStats, ListenerHandle, NodeType, Renderer, TreeRenderer};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

// ---------------------------------------------------------------------------
// Headless DOM
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum HNode {
    Element(Rc<HeadlessElement>),
    Text(Rc<TextInner>),
}

pub struct HeadlessElement {
    pub tag: String,
    pub attrs: RefCell<Vec<(String, AttrValue)>>,
    listeners: RefCell<Vec<(u64, String, ferrum_core::EventHandler)>>,
    children: RefCell<Vec<HNode>>,
    parent: RefCell<Option<Weak<HeadlessElement>>>,
}

pub struct TextInner {
    value: RefCell<String>,
    parent: RefCell<Option<Weak<HeadlessElement>>>,
}

impl HeadlessElement {
    fn new(tag: &str) -> Rc<Self> {
        Rc::new(HeadlessElement {
            tag: tag.to_string(),
            attrs: RefCell::new(Vec::new()),
            listeners: RefCell::new(Vec::new()),
            children: RefCell::new(Vec::new()),
            parent: RefCell::new(None),
        })
    }
}

fn set_parent(node: &HNode, parent: &Rc<HeadlessElement>) {
    match node {
        HNode::Element(e) => *e.parent.borrow_mut() = Some(Rc::downgrade(parent)),
        HNode::Text(t) => *t.parent.borrow_mut() = Some(Rc::downgrade(parent)),
    }
}

fn parent_of(node: &HNode) -> Option<Rc<HeadlessElement>> {
    match node {
        HNode::Element(e) => e.parent.borrow().as_ref().and_then(|w| w.upgrade()),
        HNode::Text(t) => t.parent.borrow().as_ref().and_then(|w| w.upgrade()),
    }
}

/// The headless DOM backend.
pub struct HeadlessBackend;

impl Renderer for HeadlessBackend {
    type Node = HNode;

    fn create_element(&self, tag: &str) -> HNode {
        HNode::Element(HeadlessElement::new(tag))
    }

    fn create_text(&self, text: &str) -> HNode {
        HNode::Text(Rc::new(TextInner {
            value: RefCell::new(text.to_string()),
            parent: RefCell::new(None),
        }))
    }

    fn set_attribute(&self, node: &HNode, name: &str, value: &str) {
        if let HNode::Element(e) = node {
            let mut attrs = e.attrs.borrow_mut();
            match attrs.iter_mut().find(|(n, _)| n == name) {
                Some(slot) => slot.1 = AttrValue::Str(value.to_string()),
                None => attrs.push((name.to_string(), AttrValue::Str(value.to_string()))),
            }
        }
    }

    fn set_property(&self, node: &HNode, name: &str, value: &AttrValue) {
        if let HNode::Element(e) = node {
            let mut attrs = e.attrs.borrow_mut();
            match attrs.iter_mut().find(|(n, _)| n == name) {
                Some(slot) => slot.1 = value.clone(),
                None => attrs.push((name.to_string(), value.clone())),
            }
        }
    }

    fn remove_attribute(&self, node: &HNode, name: &str) {
        if let HNode::Element(e) = node {
            e.attrs.borrow_mut().retain(|(n, _)| n != name);
        }
    }

    fn set_text(&self, node: &HNode, text: &str) {
        if let HNode::Text(t) = node {
            *t.value.borrow_mut() = text.to_string();
        }
    }

    fn append_child(&self, parent: &HNode, child: &HNode) {
        if let HNode::Element(p) = parent {
            detach(child);
            p.children.borrow_mut().push(child.clone());
            set_parent(child, p);
        }
    }

    fn insert_before(&self, parent: &HNode, child: &HNode, reference: Option<&HNode>) {
        if let HNode::Element(p) = parent {
            detach(child);
            let index = reference
                .and_then(|r| {
                    p.children
                        .borrow()
                        .iter()
                        .position(|c| HeadlessBackend::node_eq(c, r))
                })
                .unwrap_or(p.children.borrow().len());
            p.children.borrow_mut().insert(index, child.clone());
            set_parent(child, p);
        }
    }

    fn remove_child(&self, parent: &HNode, child: &HNode) {
        if let HNode::Element(p) = parent {
            p.children
                .borrow_mut()
                .retain(|c| !HeadlessBackend::node_eq(c, child));
        }
        if let Some(p) = parent_of(child) {
            let _ = p;
        }
        match child {
            HNode::Element(e) => *e.parent.borrow_mut() = None,
            HNode::Text(t) => *t.parent.borrow_mut() = None,
        }
    }

    fn next_sibling(&self, node: &HNode) -> Option<HNode> {
        let parent = parent_of(node)?;
        let children = parent.children.borrow();
        let pos = children
            .iter()
            .position(|c| HeadlessBackend::node_eq(c, node))?;
        children.get(pos + 1).cloned()
    }

    fn node_eq(a: &HNode, b: &HNode) -> bool {
        match (a, b) {
            (HNode::Element(x), HNode::Element(y)) => Rc::ptr_eq(x, y),
            (HNode::Text(x), HNode::Text(y)) => Rc::ptr_eq(x, y),
            _ => false,
        }
    }

    fn node_type(&self, node: &HNode) -> NodeType {
        match node {
            HNode::Element(e) => NodeType::Element(e.tag.clone()),
            HNode::Text(t) => NodeType::Text(t.value.borrow().clone()),
        }
    }

    fn first_child(&self, node: &HNode) -> Option<HNode> {
        if let HNode::Element(e) = node {
            return e.children.borrow().first().cloned();
        }
        None
    }

    fn replace_child(&self, parent: &HNode, old: &HNode, new: &HNode) {
        if let HNode::Element(p) = parent {
            let mut children = p.children.borrow_mut();
            if let Some(pos) = children
                .iter()
                .position(|c| HeadlessBackend::node_eq(c, old))
            {
                children[pos] = new.clone();
            } else {
                children.push(new.clone());
            }
            drop(children);
            set_parent(new, p);
            match new {
                HNode::Element(e) => *e.parent.borrow_mut() = Some(Rc::downgrade(p)),
                HNode::Text(t) => *t.parent.borrow_mut() = Some(Rc::downgrade(p)),
            }
            match old {
                HNode::Element(e) => *e.parent.borrow_mut() = None,
                HNode::Text(t) => *t.parent.borrow_mut() = None,
            }
        }
    }

    fn add_listener(
        &self,
        node: &HNode,
        name: &str,
        handler: ferrum_core::EventHandler,
    ) -> ListenerHandle {
        let id = next_listener_id();
        if let HNode::Element(e) = node {
            e.listeners
                .borrow_mut()
                .push((id, name.to_string(), handler));
        }
        ListenerHandle(Box::new(id))
    }

    fn remove_listener(&self, node: &HNode, handle: ListenerHandle) {
        let id = *handle
            .0
            .downcast_ref::<u64>()
            .expect("listener handle type");
        if let HNode::Element(e) = node {
            e.listeners.borrow_mut().retain(|(i, _, _)| *i != id);
        }
    }
}

fn detach(child: &HNode) {
    if let Some(p) = parent_of(child) {
        p.children
            .borrow_mut()
            .retain(|c| !HeadlessBackend::node_eq(c, child));
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
// TestRenderer
// ---------------------------------------------------------------------------

struct TestInner {
    tree: TreeRenderer<HeadlessBackend>,
    render_fn: Option<Box<dyn FnMut() -> VNode>>,
    depth: usize,
}

/// A headless host: render loop + effect flushing + event firing + queries.
#[derive(Clone)]
pub struct TestRenderer {
    inner: Rc<RefCell<TestInner>>,
    root: HNode,
}

impl TestRenderer {
    pub fn new() -> Self {
        Self::with_root(HNode::Element(HeadlessElement::new("#root")))
    }

    /// Create a host around an existing root (used by hydration tests: the
    /// "server" renderer's DOM is handed to a fresh client host).
    pub fn with_root(root: HNode) -> Self {
        let effects: EffectQueue = Rc::new(RefCell::new(Vec::new()));
        let tree = TreeRenderer::new(HeadlessBackend, root.clone(), effects);
        let renderer = TestRenderer {
            inner: Rc::new(RefCell::new(TestInner {
                tree,
                render_fn: None,
                depth: 0,
            })),
            root,
        };
        let weak = Rc::downgrade(&renderer.inner);
        let rr: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(inner) = weak.upgrade() {
                TestRenderer::rerender(&inner);
            }
        });
        renderer.inner.borrow_mut().tree.set_re_render(rr);
        renderer
    }

    /// Attach to an existing DOM (server HTML) instead of mounting fresh.
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

    /// Mount the tree produced by `f`.
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

    /// Re-run the render closure (diff into the headless DOM).
    pub fn re_render(&self) {
        TestRenderer::rerender(&self.inner);
    }

    fn rerender(inner: &Rc<RefCell<TestInner>>) {
        {
            let mut i = inner.borrow_mut();
            if i.depth >= 50 {
                panic!("Ferrum: maximum update depth exceeded (50 nested renders)");
            }
            i.depth += 1;
            let TestInner {
                tree, render_fn, ..
            } = &mut *i;
            if let Some(f) = render_fn.as_mut() {
                let vnode = f();
                tree.render(&vnode);
            }
        }
        let effects = inner.borrow().tree.effects().clone();
        ferrum_core::hooks::flush_effects(&effects);
        inner.borrow_mut().depth -= 1;
    }

    // -- events ---------------------------------------------------------------

    /// Fire the `nth` listener named `event` (0-based) with an empty payload.
    /// `event` may be given with or without the `on` prefix (`"click"` or
    /// `"onclick"`).
    pub fn fire(&self, event: &str, nth: usize) {
        self.fire_with(event, nth, Box::new(()));
    }

    /// Fire the `nth` listener named `event` with a payload (downcast by the
    /// handler via `FerrumEvent::downcast`).
    pub fn fire_with(&self, event: &str, nth: usize, payload: Box<dyn Any>) {
        let handlers = self.collect_listeners(event);
        let (_, handler) = handlers
            .into_iter()
            .nth(nth)
            .unwrap_or_else(|| panic!("no listener `{}` #{}", event, nth));
        let fev = FerrumEvent::new(event, payload);
        handler(&fev);
    }

    fn collect_listeners(&self, event: &str) -> Vec<(HNode, ferrum_core::EventHandler)> {
        let mut out = Vec::new();
        collect_listeners_rec(&self.root, event, &mut out);
        out
    }
    // -- queries ---------------------------------------------------------------

    /// All elements with the given tag, in document order.
    pub fn find_all(&self, tag: &str) -> Vec<HNode> {
        let mut out = Vec::new();
        find_all_rec(&self.root, tag, &mut out);
        out
    }

    /// The root's text content.
    pub fn text(&self) -> String {
        text_content(&self.root)
    }

    /// The text content of a specific node.
    pub fn text_of(&self, node: &HNode) -> String {
        text_content(node)
    }

    /// The root's serialized HTML.
    pub fn html(&self) -> String {
        let mut s = String::new();
        html_rec(&self.root, &mut s);
        s
    }

    /// Read an attribute/property of an element node.
    pub fn attr(&self, node: &HNode, name: &str) -> Option<AttrValue> {
        if let HNode::Element(e) = node {
            return e
                .attrs
                .borrow()
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone());
        }
        None
    }

    /// The root element node.
    pub fn root(&self) -> HNode {
        self.root.clone()
    }
}

impl Default for TestRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn collect_listeners_rec(
    node: &HNode,
    event: &str,
    out: &mut Vec<(HNode, ferrum_core::EventHandler)>,
) {
    if let HNode::Element(e) = node {
        for (_, name, handler) in e.listeners.borrow().iter() {
            if name == event || name.strip_prefix("on") == Some(event) {
                out.push((node.clone(), handler.clone()));
            }
        }
        for child in e.children.borrow().iter() {
            collect_listeners_rec(child, event, out);
        }
    }
}

fn find_all_rec(node: &HNode, tag: &str, out: &mut Vec<HNode>) {
    if let HNode::Element(e) = node {
        if e.tag == tag {
            out.push(node.clone());
        }
        for child in e.children.borrow().iter() {
            find_all_rec(child, tag, out);
        }
    }
}

fn text_content(node: &HNode) -> String {
    match node {
        HNode::Element(e) => {
            let mut s = String::new();
            for c in e.children.borrow().iter() {
                s.push_str(&text_content(c));
            }
            s
        }
        HNode::Text(t) => t.value.borrow().clone(),
    }
}

fn html_rec(node: &HNode, out: &mut String) {
    match node {
        HNode::Element(e) => {
            out.push('<');
            out.push_str(&e.tag);
            for (k, v) in e.attrs.borrow().iter() {
                out.push(' ');
                out.push_str(k);
                out.push_str("=\"");
                out.push_str(&v.to_string());
                out.push('"');
            }
            out.push('>');
            for c in e.children.borrow().iter() {
                html_rec(c, out);
            }
            out.push_str("</");
            out.push_str(&e.tag);
            out.push('>');
        }
        HNode::Text(t) => out.push_str(&t.value.borrow()),
    }
}
