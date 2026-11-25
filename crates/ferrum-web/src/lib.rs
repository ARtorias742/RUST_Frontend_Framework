//! # ferrum-web
//!
//! Web renderer: a [`Renderer`] backend over `web-sys` + the shared
//! [`TreeRenderer`] reconciler, plus [`AppHandle`], the host that owns the
//! render loop and flushes effects after each commit.

use ferrum_core::hooks::EffectQueue;
use ferrum_core::{AttrValue, FerrumEvent, VNode};
use ferrum_reconciler::{HydrationStats, ListenerHandle, NodeType, Renderer, TreeRenderer};
use ferrum_router::History;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Element, Event, Node};

/// The `web-sys` DOM backend.
pub struct WebBackend;

fn document() -> web_sys::Document {
    web_sys::window()
        .expect("no window")
        .document()
        .expect("no document")
}

fn as_element(node: &Node) -> &Element {
    node.dyn_ref::<Element>()
        .expect("backend node is not an element")
}

impl Renderer for WebBackend {
    type Node = Node;

    fn create_element(&self, tag: &str) -> Node {
        document()
            .create_element(tag)
            .expect("create_element")
            .into()
    }

    fn create_text(&self, text: &str) -> Node {
        document().create_text_node(text).into()
    }

    fn set_attribute(&self, node: &Node, name: &str, value: &str) {
        as_element(node).set_attribute(name, value).ok();
    }

    fn set_property(&self, node: &Node, name: &str, value: &AttrValue) {
        let js = attr_to_js(value);
        let _ = js_sys::Reflect::set(node.as_ref(), &name.into(), &js);
    }

    fn remove_attribute(&self, node: &Node, name: &str) {
        as_element(node).remove_attribute(name).ok();
    }

    fn set_text(&self, node: &Node, text: &str) {
        node.set_node_value(Some(text));
    }

    fn append_child(&self, parent: &Node, child: &Node) {
        parent.append_child(child).ok();
    }

    fn insert_before(&self, parent: &Node, child: &Node, reference: Option<&Node>) {
        parent.insert_before(child, reference).ok();
    }

    fn remove_child(&self, parent: &Node, child: &Node) {
        parent.remove_child(child).ok();
    }

    fn next_sibling(&self, node: &Node) -> Option<Node> {
        node.next_sibling()
    }

    fn node_eq(a: &Node, b: &Node) -> bool {
        a == b
    }

    fn node_type(&self, node: &Node) -> NodeType {
        if node.node_type() == web_sys::Node::ELEMENT_NODE {
            if let Some(el) = node.dyn_ref::<Element>() {
                return NodeType::Element(el.tag_name().to_lowercase());
            }
        }
        if node.node_type() == web_sys::Node::TEXT_NODE {
            return NodeType::Text(node.node_value().unwrap_or_default());
        }
        NodeType::Other
    }

    fn first_child(&self, node: &Node) -> Option<Node> {
        node.first_child()
    }

    fn replace_child(&self, parent: &Node, old: &Node, new: &Node) {
        parent.replace_child(new, old).ok();
    }

    fn add_listener(&self, node: &Node, name: &str, handler: ferrum_core::EventHandler) -> ListenerHandle {
        let closure = Closure::<dyn FnMut(Event)>::new(move |ev: Event| {
            let fev = FerrumEvent::new(ev.type_(), Box::new(ev));
            handler(&fev);
        });
        let function: &js_sys::Function = closure.as_ref().unchecked_ref();
        node.add_event_listener_with_callback(name, function)
            .expect("add_event_listener");
        ListenerHandle(Box::new((name.to_string(), closure)))
    }

    fn remove_listener(&self, node: &Node, handle: ListenerHandle) {
        let boxed = handle
            .0
            .downcast::<(String, Closure<dyn FnMut(Event)>)>()
            .expect("listener handle type");
        let (name, closure) = *boxed;
        let function: &js_sys::Function = closure.as_ref().unchecked_ref();
        node.remove_event_listener_with_callback(&name, function)
            .ok();
        drop(closure);
    }
}

fn attr_to_js(value: &AttrValue) -> wasm_bindgen::JsValue {
    match value {
        AttrValue::Str(s) => wasm_bindgen::JsValue::from_str(s),
        AttrValue::Bool(b) => wasm_bindgen::JsValue::from_bool(*b),
        AttrValue::Int(i) => wasm_bindgen::JsValue::from_f64(*i as f64),
        AttrValue::Float(x) => wasm_bindgen::JsValue::from_f64(*x),
    }
}

// ---------------------------------------------------------------------------
// BrowserHistory
// ---------------------------------------------------------------------------

/// Browser history backed by `history.pushState` + `popstate`.
type Subs = Rc<RefCell<Vec<Rc<dyn Fn()>>>>;

pub struct BrowserHistory {
    subs: Subs,
}

impl BrowserHistory {
    pub fn new() -> Rc<Self> {
        let history = Rc::new(BrowserHistory {
            subs: Rc::new(RefCell::new(Vec::new())),
        });
        // popstate -> notify subscribers (kept alive by the closure).
        let subs = history.subs.clone();
        let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
            for cb in subs.borrow().iter() {
                cb();
            }
        });
        let function: &js_sys::Function = closure.as_ref().unchecked_ref();
        web_sys::window()
            .expect("no window")
            .add_event_listener_with_callback("popstate", function)
            .expect("add popstate listener");
        closure.forget(); // lives for the page lifetime
        history
    }
}

impl History for BrowserHistory {
    fn current(&self) -> String {
        web_sys::window()
            .expect("no window")
            .location()
            .pathname()
            .unwrap_or_else(|_| "/".to_string())
    }

    fn push(&self, url: &str) {
        let window = web_sys::window().expect("no window");
        window
            .history()
            .expect("no history")
            .push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(url))
            .expect("push_state");
    }

    fn replace(&self, url: &str) {
        let window = web_sys::window().expect("no window");
        window
            .history()
            .expect("no history")
            .replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(url))
            .expect("replace_state");
    }

    fn back(&self) {
        let _ = web_sys::window()
            .expect("no window")
            .history()
            .expect("no history")
            .back();
    }

    fn subscribe(&self, cb: Rc<dyn Fn()>) {
        self.subs.borrow_mut().push(cb);
    }
}

/// Convenience helpers on [`FerrumEvent`] for web handlers.
pub trait FerrumEventExt {
    /// Call `preventDefault()` on the underlying DOM event.
    fn prevent_default(&self);
    /// The event target, if it is an element.
    fn target_element(&self) -> Option<Element>;
    /// The target's value (input/select/textarea), if any.
    fn target_value(&self) -> Option<String>;
}

impl FerrumEventExt for FerrumEvent {
    fn prevent_default(&self) {
        if let Some(ev) = self.downcast::<web_sys::Event>() {
            ev.prevent_default();
        }
    }

    fn target_element(&self) -> Option<Element> {
        self.downcast::<web_sys::Event>()?
            .target()?
            .dyn_into::<Element>()
            .ok()
    }

    fn target_value(&self) -> Option<String> {
        let el = self.target_element()?;
        if let Some(input) = el.dyn_ref::<web_sys::HtmlInputElement>() {
            return Some(input.value());
        }
        if let Some(select) = el.dyn_ref::<web_sys::HtmlSelectElement>() {
            return Some(select.value());
        }
        None
    }
}

// ---------------------------------------------------------------------------
// WebRenderer + AppHandle
// ---------------------------------------------------------------------------

/// Mounts and updates a [`VNode`] tree under a root DOM element.
pub struct WebRenderer {
    inner: TreeRenderer<WebBackend>,
}

impl WebRenderer {
    pub fn new(root: Element) -> Self {
        let effects: EffectQueue = Rc::new(RefCell::new(Vec::new()));
        WebRenderer {
            inner: TreeRenderer::new(WebBackend, root.into(), effects),
        }
    }

    pub fn set_re_render(&mut self, f: Rc<dyn Fn()>) {
        self.inner.set_re_render(f);
    }

    /// Render (mount or update) `vnode` under the root element.
    pub fn render(&mut self, vnode: &VNode) {
        self.inner.render(vnode);
    }

    /// Attach to existing DOM (server-rendered HTML) instead of mounting.
    pub fn hydrate(&mut self, vnode: &VNode) -> HydrationStats {
        self.inner.hydrate(vnode)
    }

    /// Run queued effects (call after `render`).
    pub fn flush_effects(&self) {
        self.inner.flush_effects();
    }

    /// The shared effect queue (cloned by the host before flushing, so an
    /// effect may re-enter the render loop without a `RefCell` conflict).
    pub fn effects(&self) -> &EffectQueue {
        self.inner.effects()
    }

    /// Unmount everything (effect cleanups run).
    pub fn unmount(&mut self) {
        self.inner.unmount();
    }
}

struct AppInner {
    renderer: WebRenderer,
    render_fn: Option<Box<dyn FnMut() -> VNode>>,
    depth: usize,
}

/// Cheaply-cloneable handle that owns the renderer and a user render closure.
///
/// Call [`AppHandle::render`] once to mount, then call [`AppHandle::re_render`]
/// from event handlers or state setters to update. Hooks (Phase 3) use this
/// re-render path via a `Weak` link — no `Rc` cycles.
#[derive(Clone)]
pub struct AppHandle {
    inner: Rc<RefCell<AppInner>>,
}

/// Inject (or update) the `<style id="ferrum-styles">` tag with the current
/// scoped stylesheet (all `css!`/`css()` rules registered so far).
pub fn inject_style_tag(document: &web_sys::Document) {
    let style_el = match document.get_element_by_id("ferrum-styles") {
        Some(el) => el,
        None => {
            let el = document.create_element("style").expect("create style");
            el.set_id("ferrum-styles");
            document
                .head()
                .expect("no head")
                .append_child(&el)
                .expect("append style");
            el
        }
    };
    style_el.set_text_content(Some(&ferrum_styling::stylesheet()));
}

thread_local! {
    static STYLE_VERSION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Re-inject the style tag if new `css!` rules were registered since the
/// last sync (cheap: only touches the DOM when the registry changed).
fn sync_styles() {
    let v = ferrum_styling::registry_version();
    STYLE_VERSION.with(|sv| {
        if sv.get() != v {
            sv.set(v);
            inject_style_tag(&document());
        }
    });
}

impl AppHandle {
    pub fn new(root: Element) -> Self {
        inject_style_tag(&document());
        let app = AppHandle {
            inner: Rc::new(RefCell::new(AppInner {
                renderer: WebRenderer::new(root),
                render_fn: None,
                depth: 0,
            })),
        };
        // Wire the hook re-render callback (Weak: no cycle).
        let weak = Rc::downgrade(&app.inner);
        let rr: Rc<dyn Fn()> = Rc::new(move || {
            if let Some(inner) = weak.upgrade() {
                AppHandle::rerender(&inner);
            }
        });
        app.inner.borrow_mut().renderer.set_re_render(rr);
        app
    }

    /// Mount the tree produced by `f` and store `f` for future re-renders.
    pub fn render(&self, mut f: impl FnMut() -> VNode + 'static) {
        {
            let mut inner = self.inner.borrow_mut();
            let vnode = f();
            inner.render_fn = Some(Box::new(f));
            inner.renderer.render(&vnode);
        }
        sync_styles();
        let effects = self.inner.borrow().renderer.effects().clone();
        ferrum_core::hooks::flush_effects(&effects);
    }

    /// Attach to server-rendered DOM instead of mounting fresh.
    pub fn hydrate(&self, mut f: impl FnMut() -> VNode + 'static) -> HydrationStats {
        let stats = {
            let mut inner = self.inner.borrow_mut();
            let vnode = f();
            inner.render_fn = Some(Box::new(f));
            inner.renderer.hydrate(&vnode)
        };
        sync_styles();
        let effects = self.inner.borrow().renderer.effects().clone();
        ferrum_core::hooks::flush_effects(&effects);
        stats
    }

    /// Re-run the stored render closure and diff the result into the DOM.
    pub fn re_render(&self) {
        AppHandle::rerender(&self.inner);
    }

    /// Unmount the tree (effect cleanups run).
    pub fn unmount(&self) {
        self.inner.borrow_mut().renderer.unmount();
    }

    fn rerender(inner: &Rc<RefCell<AppInner>>) {
        {
            let mut i = inner.borrow_mut();
            if i.depth >= 50 {
                panic!(
                    "Ferrum: maximum update depth exceeded (50 nested renders). \
                     An effect or state setter is updating state in a loop."
                );
            }
            i.depth += 1;
            let AppInner {
                renderer, render_fn, ..
            } = &mut *i;
            if let Some(f) = render_fn.as_mut() {
                let vnode = f();
                renderer.render(&vnode);
            }
        }
        sync_styles();
        // Effects run after the commit borrow is dropped: they may re-enter
        // `rerender` (nested update) safely.
        let effects = inner.borrow().renderer.effects().clone();
        ferrum_core::hooks::flush_effects(&effects);
        inner.borrow_mut().depth -= 1;
    }
}