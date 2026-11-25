//! Generic tree renderer: mount/update/unmount of [`VNode`] trees over a
//! pluggable [`Renderer`] backend (the custom-renderer API, React
//! Reconciler-style).
//!
//! `ferrum-web` implements [`Renderer`] with `web-sys`; `ferrum-testing`
//! implements it with a headless DOM. Both share this reconciler, so keyed
//! diffing, component instances, hooks, effects, and context behave
//! identically everywhere.

use ferrum_core::hooks::{render_with, EffectQueue, Instance};
use ferrum_core::{AttrValue, EventHandler, Key, VNode};
use std::any::Any;
use std::rc::Rc;

use crate::plan_children;
use crate::ChildOp;

/// Opaque per-listener handle returned by [`Renderer::add_listener`].
pub struct ListenerHandle(pub Box<dyn Any>);

/// The kind of a backend node, used by hydration to match server HTML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Element(String),
    Text(String),
    Other,
}

/// A DOM backend. Implementations must be cheap to clone/share (`&self`).
pub trait Renderer {
    type Node: Clone;

    fn create_element(&self, tag: &str) -> Self::Node;
    fn create_text(&self, text: &str) -> Self::Node;
    fn set_attribute(&self, node: &Self::Node, name: &str, value: &str);
    /// Set a DOM *property* (used for `value`/`checked` on form controls).
    fn set_property(&self, node: &Self::Node, name: &str, value: &AttrValue);
    fn remove_attribute(&self, node: &Self::Node, name: &str);
    fn set_text(&self, node: &Self::Node, text: &str);
    fn append_child(&self, parent: &Self::Node, child: &Self::Node);
    fn insert_before(&self, parent: &Self::Node, child: &Self::Node, reference: Option<&Self::Node>);
    fn remove_child(&self, parent: &Self::Node, child: &Self::Node);
    fn next_sibling(&self, node: &Self::Node) -> Option<Self::Node>;
    fn node_eq(a: &Self::Node, b: &Self::Node) -> bool;

    // -- hydration support ---------------------------------------------------

    /// Classify a node (element tag / text value) for hydration matching.
    fn node_type(&self, node: &Self::Node) -> NodeType;
    /// First DOM child, if any.
    fn first_child(&self, node: &Self::Node) -> Option<Self::Node>;
    /// Replace `old` with `new` in `parent` (same position).
    fn replace_child(&self, parent: &Self::Node, old: &Self::Node, new: &Self::Node);

    fn add_listener(&self, node: &Self::Node, name: &str, handler: EventHandler) -> ListenerHandle;
    fn remove_listener(&self, node: &Self::Node, handle: ListenerHandle);
}

/// Hydration counters (server HTML → live DOM attach + mismatch recovery).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HydrationStats {
    pub matched: usize,
    pub text_fixed: usize,
    pub tag_replaced: usize,
    pub children_appended: usize,
    pub children_removed: usize,
}

// ---------------------------------------------------------------------------
// Mounted tree
// ---------------------------------------------------------------------------

enum MountedNode<N: Clone> {
    Element(MountedElement<N>),
    Text { node: N, text: String },
    Component {
        instance: Rc<Instance>,
        rendered: Box<MountedNode<N>>,
    },
    Fragment(Vec<MountedNode<N>>),
    Provider(Vec<MountedNode<N>>),
    Empty,
}

struct MountedElement<N: Clone> {
    node: N,
    tag: String,
    attrs: Vec<(String, AttrValue)>,
    listeners: Vec<(String, ListenerHandle)>,
    children: Vec<MountedNode<N>>,
    key: Option<Key>,
}

impl<N: Clone> MountedNode<N> {
    fn key(&self) -> Option<Key> {
        match self {
            MountedNode::Element(e) => e.key,
            _ => None,
        }
    }
}

/// Renders a VNode tree into a backend, updating in place on re-render.
pub struct TreeRenderer<B: Renderer> {
    backend: B,
    root: B::Node,
    re_render: Rc<dyn Fn()>,
    effects: EffectQueue,
    current: Option<MountedNode<B::Node>>,
    /// Stack of enclosing component instance ids (DevTools parent links).
    component_stack: std::cell::RefCell<Vec<u64>>,
}

impl<B: Renderer> Drop for TreeRenderer<B> {
    fn drop(&mut self) {
        self.unmount();
    }
}

impl<B: Renderer> TreeRenderer<B> {
    pub fn new(backend: B, root: B::Node, effects: EffectQueue) -> Self {
        TreeRenderer {
            backend,
            root,
            re_render: Rc::new(|| {}),
            effects,
            current: None,
            component_stack: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn set_re_render(&mut self, f: Rc<dyn Fn()>) {
        self.re_render = f;
    }

    pub fn effects(&self) -> &EffectQueue {
        &self.effects
    }

    /// Run queued effects (call after `render`, outside any borrow).
    pub fn flush_effects(&self) {
        ferrum_core::hooks::flush_effects(&self.effects);
    }

    /// Mount (first call) or update (subsequent calls) `vnode` under the root.
    pub fn render(&mut self, vnode: &VNode) {
        match self.current.take() {
            None => {
                let m = self.mount(vnode, &self.root.clone());
                self.current = Some(m);
            }
            Some(mut old) => {
                let root = self.root.clone();
                self.update(&root, &mut old, vnode);
                self.current = Some(old);
            }
        }
    }

    /// Attach to existing backend nodes (server-rendered HTML) instead of
    /// creating them. Matching is positional; fragments/providers are
    /// transparent. Text mismatches are fixed in place; tag/type mismatches
    /// replace only the affected subtree (client re-render recovery).
    pub fn hydrate(&mut self, vnode: &VNode) -> HydrationStats {
        let root = self.root.clone();
        let mut stats = HydrationStats::default();
        let mut cursor = self.backend.first_child(&root);
        let mounted = self.hydrate_vnode(&root, &mut cursor, vnode, &mut stats);
        while let Some(node) = cursor.take() {
            self.backend.remove_child(&root, &node);
            stats.children_removed += 1;
            cursor = self.backend.next_sibling(&node);
        }
        self.current = Some(mounted);
        stats
    }

    /// Unmount everything (run effect cleanups, detach nodes).
    pub fn unmount(&mut self) {
        if let Some(mut m) = self.current.take() {
            let root = self.root.clone();
            self.unmount_node(&mut m, &root);
        }
    }

    // -- mount ---------------------------------------------------------------

    fn mount(&self, vnode: &VNode, parent: &B::Node) -> MountedNode<B::Node> {
        match vnode {
            VNode::Element(e) => {
                let node = self.backend.create_element(&e.tag);
                let mut m = MountedElement {
                    node: node.clone(),
                    tag: e.tag.clone(),
                    attrs: Vec::new(),
                    listeners: Vec::new(),
                    children: Vec::new(),
                    key: e.key,
                };
                for (name, value) in &e.attrs {
                    set_attr(&self.backend, &m.node, name, value);
                    m.attrs.push((name.clone(), value.clone()));
                }
                for (name, handler) in &e.listeners {
                    let handle = self.backend.add_listener(&m.node, name, handler.clone());
                    m.listeners.push((name.clone(), handle));
                }
                for child in &e.children {
                    let mounted = self.mount(child, &m.node.clone());
                    m.children.push(mounted);
                }
                self.backend.append_child(parent, &m.node);
                MountedNode::Element(m)
            }
            VNode::Text(t) => {
                let node = self.backend.create_text(t);
                self.backend.append_child(parent, &node);
                MountedNode::Text {
                    node,
                    text: t.clone(),
                }
            }
            VNode::Component(c) => {
                let instance = Instance::new(c.name);
                let comp_parent = self.component_stack.borrow().last().copied();
                self.component_stack.borrow_mut().push(instance.id);
                let rendered_vnode = render_with(
                    &instance,
                    self.re_render.clone(),
                    self.effects.clone(),
                    || c.props.render(),
                );
                let rendered = self.mount(&rendered_vnode, parent);
                self.component_stack.borrow_mut().pop();
                crate::devtools::report_mount(
                    instance.id,
                    instance.name,
                    comp_parent,
                    &instance.hook_infos(),
                );
                MountedNode::Component {
                    instance,
                    rendered: Box::new(rendered),
                }
            }
            VNode::Fragment(children) => {
                let mut mounted = Vec::with_capacity(children.len());
                for child in children {
                    mounted.push(self.mount(child, parent));
                }
                MountedNode::Fragment(mounted)
            }
            VNode::Provider(p) => {
                ferrum_core::hooks::push_provider(p.ctx_id, p.value.clone());
                let mut mounted = Vec::with_capacity(p.children.len());
                for child in &p.children {
                    mounted.push(self.mount(child, parent));
                }
                ferrum_core::hooks::pop_provider();
                MountedNode::Provider(mounted)
            }
            VNode::Empty => MountedNode::Empty,
        }
    }

    // -- update --------------------------------------------------------------

    fn update(&self, parent: &B::Node, old: &mut MountedNode<B::Node>, new: &VNode) {
        match (&mut *old, new) {
            (MountedNode::Element(oe), VNode::Element(ne)) if oe.tag == ne.tag => {
                update_attrs(&self.backend, oe, ne);
                update_listeners(&self.backend, oe, ne);
                let node = oe.node.clone();
                self.update_children(&node, &mut oe.children, &ne.children);
            }
            (MountedNode::Text { node, text }, VNode::Text(t)) => {
                if text != t {
                    self.backend.set_text(node, t);
                    *text = t.clone();
                }
            }
            (MountedNode::Component { instance, rendered }, VNode::Component(nc))
                if instance.name == nc.name =>
            {
                self.component_stack.borrow_mut().push(instance.id);
                let r = render_with(
                    instance,
                    self.re_render.clone(),
                    self.effects.clone(),
                    || nc.props.render(),
                );
                self.update(parent, rendered, &r);
                self.component_stack.borrow_mut().pop();
                crate::devtools::report_render(instance.id, instance.name, &instance.hook_infos());
            }
            (MountedNode::Fragment(oc), VNode::Fragment(nc)) => {
                self.update_children(parent, oc, nc);
            }
            (MountedNode::Provider(oc), VNode::Provider(nc)) => {
                ferrum_core::hooks::push_provider(nc.ctx_id, nc.value.clone());
                self.update_children(parent, oc, &nc.children);
                ferrum_core::hooks::pop_provider();
            }
            _ => {
                // Type/name changed: replace wholesale.
                self.unmount_node(old, parent);
                *old = self.mount(new, parent);
            }
        }
    }

    fn update_children(
        &self,
        parent: &B::Node,
        old: &mut Vec<MountedNode<B::Node>>,
        new: &[VNode],
    ) {
        let old_keys: Vec<Option<Key>> = old.iter().map(|m| m.key()).collect();
        let new_keys: Vec<Option<Key>> = new.iter().map(|v| v.key()).collect();
        let plan = plan_children(&old_keys, &new_keys);

        let mut old_used = vec![false; old.len()];
        let mut old_slots: Vec<Option<MountedNode<B::Node>>> = old.drain(..).map(Some).collect();
        let mut mounted: Vec<MountedNode<B::Node>> = Vec::with_capacity(new.len());

        for (i, vnode) in new.iter().enumerate() {
            match plan.ops[i] {
                ChildOp::Reuse(j) => {
                    old_used[j] = true;
                    let mut slot = old_slots[j].take().unwrap();
                    self.update(parent, &mut slot, vnode);
                    mounted.push(slot);
                }
                ChildOp::Mount => {
                    mounted.push(self.mount(vnode, parent));
                }
            }
        }
        for (j, slot) in old_slots.iter_mut().enumerate() {
            if !old_used[j] {
                if let Some(mut m) = slot.take() {
                    self.unmount_node(&mut m, parent);
                }
            }
        }
        *old = mounted;

        // Reorder DOM nodes to match the new order (no-op when already
        // positioned, to preserve focus).
        let nodes: Vec<B::Node> = old.iter().flat_map(|m| m.flat()).collect();
        for i in 0..nodes.len() {
            let node = &nodes[i];
            let reference = nodes.get(i + 1);
            let correct = match (self.backend.next_sibling(node), reference) {
                (Some(ns), Some(r)) => B::node_eq(&ns, r),
                (None, None) => true,
                _ => false,
            };
            if !correct {
                self.backend.insert_before(parent, node, reference);
            }
        }
    }

    // -- hydration -----------------------------------------------------------

    fn hydrate_vnode(
        &self,
        parent: &B::Node,
        cursor: &mut Option<B::Node>,
        vnode: &VNode,
        stats: &mut HydrationStats,
    ) -> MountedNode<B::Node> {
        match vnode {
            VNode::Element(e) => {
                let (node, reused) = match cursor.take() {
                    Some(old) => {
                        let matches = matches!(
                            self.backend.node_type(&old),
                            NodeType::Element(ref tag) if tag == &e.tag
                        );
                        if matches {
                            (old, true)
                        } else {
                            let fresh = self.backend.create_element(&e.tag);
                            self.backend.replace_child(parent, &old, &fresh);
                            stats.tag_replaced += 1;
                            (fresh, false)
                        }
                    }
                    None => {
                        let fresh = self.backend.create_element(&e.tag);
                        self.backend.append_child(parent, &fresh);
                        stats.children_appended += 1;
                        (fresh, false)
                    }
                };
                if reused {
                    stats.matched += 1;
                }

                let mut mounted = MountedElement {
                    node: node.clone(),
                    tag: e.tag.clone(),
                    attrs: Vec::new(),
                    listeners: Vec::new(),
                    children: Vec::new(),
                    key: e.key,
                };
                for (name, value) in &e.attrs {
                    set_attr(&self.backend, &mounted.node, name, value);
                    mounted.attrs.push((name.clone(), value.clone()));
                }
                for (name, handler) in &e.listeners {
                    let handle = self.backend.add_listener(&mounted.node, name, handler.clone());
                    mounted.listeners.push((name.clone(), handle));
                }

                let mut child_cursor = self.backend.first_child(&node);
                for child in &e.children {
                    let mounted_child =
                        self.hydrate_vnode(&node, &mut child_cursor, child, stats);
                    mounted.children.push(mounted_child);
                }
                while let Some(extra) = child_cursor.take() {
                    self.backend.remove_child(&node, &extra);
                    stats.children_removed += 1;
                    child_cursor = self.backend.next_sibling(&extra);
                }
                *cursor = self.backend.next_sibling(&node);
                MountedNode::Element(mounted)
            }
            VNode::Text(text) => {
                let node = match cursor.take() {
                    Some(old) => match self.backend.node_type(&old) {
                        NodeType::Text(ref old_text) => {
                            if old_text != text {
                                self.backend.set_text(&old, text);
                                stats.text_fixed += 1;
                            } else {
                                stats.matched += 1;
                            }
                            old
                        }
                        _ => {
                            let fresh = self.backend.create_text(text);
                            self.backend.replace_child(parent, &old, &fresh);
                            stats.tag_replaced += 1;
                            fresh
                        }
                    },
                    None => {
                        let fresh = self.backend.create_text(text);
                        self.backend.append_child(parent, &fresh);
                        stats.children_appended += 1;
                        fresh
                    }
                };
                *cursor = self.backend.next_sibling(&node);
                MountedNode::Text {
                    node,
                    text: text.clone(),
                }
            }
            VNode::Component(c) => {
                let instance = Instance::new(c.name);
                let comp_parent = self.component_stack.borrow().last().copied();
                self.component_stack.borrow_mut().push(instance.id);
                let rendered_vnode = render_with(
                    &instance,
                    self.re_render.clone(),
                    self.effects.clone(),
                    || c.props.render(),
                );
                let rendered =
                    self.hydrate_vnode(parent, cursor, &rendered_vnode, stats);
                self.component_stack.borrow_mut().pop();
                crate::devtools::report_mount(
                    instance.id,
                    instance.name,
                    comp_parent,
                    &instance.hook_infos(),
                );
                MountedNode::Component {
                    instance,
                    rendered: Box::new(rendered),
                }
            }
            VNode::Fragment(children) => {
                let mut mounted = Vec::with_capacity(children.len());
                for child in children {
                    mounted.push(self.hydrate_vnode(parent, cursor, child, stats));
                }
                MountedNode::Fragment(mounted)
            }
            VNode::Provider(p) => {
                ferrum_core::hooks::push_provider(p.ctx_id, p.value.clone());
                let mut mounted = Vec::with_capacity(p.children.len());
                for child in &p.children {
                    mounted.push(self.hydrate_vnode(parent, cursor, child, stats));
                }
                ferrum_core::hooks::pop_provider();
                MountedNode::Provider(mounted)
            }
            VNode::Empty => MountedNode::Empty,
        }
    }

    // -- unmount -------------------------------------------------------------

    fn unmount_node(&self, m: &mut MountedNode<B::Node>, parent: &B::Node) {
        match m {
            MountedNode::Element(me) => {
                let node = me.node.clone();
                for child in &mut me.children {
                    self.unmount_node(child, &node);
                }
                for (_, handle) in me.listeners.drain(..) {
                    self.backend.remove_listener(&me.node, handle);
                }
                self.backend.remove_child(parent, &me.node);
            }
            MountedNode::Text { node, .. } => {
                self.backend.remove_child(parent, node);
            }
            MountedNode::Component { instance, rendered } => {
                instance.unmount(); // run effect cleanups
                crate::devtools::report_unmount(instance.id, instance.name);
                self.unmount_node(rendered, parent);
            }
            MountedNode::Fragment(children) | MountedNode::Provider(children) => {
                for child in children.iter_mut() {
                    self.unmount_node(child, parent);
                }
            }
            MountedNode::Empty => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Attribute / listener diffing (backend-independent)
// ---------------------------------------------------------------------------

fn set_attr<B: Renderer>(b: &B, node: &B::Node, name: &str, value: &AttrValue) {
    match name {
        "value" | "checked" => b.set_property(node, name, value),
        _ => b.set_attribute(node, name, &value.to_string()),
    }
}

fn update_attrs<B: Renderer>(b: &B, oe: &mut MountedElement<B::Node>, ne: &ferrum_core::VElement) {
    oe.attrs.retain(|(name, _)| {
        if ne.attrs.iter().any(|(n, _)| n == name) {
            true
        } else {
            b.remove_attribute(&oe.node, name);
            false
        }
    });
    for (name, value) in &ne.attrs {
        let changed = match oe.attrs.iter().find(|(n, _)| n == name) {
            Some((_, old)) => old != value,
            None => true,
        };
        if changed {
            set_attr(b, &oe.node, name, value);
        }
        match oe.attrs.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = value.clone(),
            None => oe.attrs.push((name.clone(), value.clone())),
        }
    }
}

fn update_listeners<B: Renderer>(
    b: &B,
    oe: &mut MountedElement<B::Node>,
    ne: &ferrum_core::VElement,
) {
    // Remove all old listeners, add all new (simple + correct; root
    // delegation is future work).
    for (name, handle) in oe.listeners.drain(..) {
        let _ = name;
        b.remove_listener(&oe.node, handle);
    }
    for (name, handler) in &ne.listeners {
        let handle = b.add_listener(&oe.node, name, handler.clone());
        oe.listeners.push((name.clone(), handle));
    }
}

/// All backend nodes a mounted node contributes, in document order.
fn flatten_nodes<N: Clone>(m: &MountedNode<N>, out: &mut Vec<N>) {
    match m {
        MountedNode::Element(e) => out.push(e.node.clone()),
        MountedNode::Text { node, .. } => out.push(node.clone()),
        MountedNode::Component { rendered, .. } => flatten_nodes(rendered, out),
        MountedNode::Fragment(children) | MountedNode::Provider(children) => {
            for c in children {
                flatten_nodes(c, out);
            }
        }
        MountedNode::Empty => {}
    }
}

trait FlattenNodes<N: Clone> {
    fn flat(&self) -> Vec<N>;
}

impl<N: Clone> FlattenNodes<N> for MountedNode<N> {
    fn flat(&self) -> Vec<N> {
        let mut out = Vec::new();
        flatten_nodes(self, &mut out);
        out
    }
}