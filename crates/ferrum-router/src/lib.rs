//! # ferrum-router
//!
//! Declarative routing: static + `:param` segments, nested layouts (with
//! outlets), redirects, and a pluggable [`History`] (browser history in
//! `ferrum-web`, in-memory [`MemoryHistory`] for tests/SSR).
//!
//! ```ignore
//! let router = Router::new(
//!     Router::rules()
//!         .route("/", Home)
//!         .redirect("/legacy", "/home")
//!         .nest("/app", AppLayout, |r| {
//!             r.route("", Dashboard).route("settings", Settings)
//!         }),
//!     history,
//! );
//! router.start();
//! // in the app render closure:
//! let m = router.current();
//! m.target.render(&m.params, m.outlet)
//! ```

use ferrum_core::VNode;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// URL parameters captured from `:param` segments.
#[derive(Debug, Clone, Default)]
pub struct Params(HashMap<String, String>);

impl Params {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(|s| s.as_str())
    }
}

/// A route's data loader (SSR contract, Phase 7): the server awaits these
/// before rendering; on the client they are typically driven by
/// `ferrum_data::use_query`.
pub type LoaderFuture = Pin<Box<dyn Future<Output = Result<(), String>>>>;

/// A route's code-splitting chunk metadata (Phase 7).
///
/// The bundler emits one wasm/js chunk per `name`; `budget_bytes` is the
/// per-route budget enforced by CI (`ferrum build --budget` / release gate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkInfo {
    pub name: String,
    pub budget_bytes: usize,
}

impl ChunkInfo {
    pub fn new(name: impl Into<String>, budget_bytes: usize) -> Self {
        ChunkInfo {
            name: name.into(),
            budget_bytes,
        }
    }

    /// Check an actual chunk size against the route budget.
    pub fn check(&self, actual_bytes: usize) -> Result<(), String> {
        if actual_bytes <= self.budget_bytes {
            Ok(())
        } else {
            Err(format!(
                "chunk `{}` is {} bytes, over the {} byte budget",
                self.name, actual_bytes, self.budget_bytes
            ))
        }
    }
}

/// What a route renders. `outlet` is the matched child's output (for layouts)
/// or `VNode::Empty` (for leaves).
pub trait RouteTarget {
    fn render(&self, params: &Params, outlet: VNode) -> VNode;
    fn loader(&self, _params: &Params) -> Option<LoaderFuture> {
        None
    }
    /// Optional code-splitting chunk for this route.
    fn chunk(&self) -> Option<ChunkInfo> {
        None
    }
}

/// History abstraction (browser, memory, SSR).
pub trait History {
    fn current(&self) -> String;
    fn push(&self, url: &str);
    #[allow(dead_code)]
    fn replace(&self, url: &str);
    #[allow(dead_code)]
    fn back(&self);
    fn subscribe(&self, cb: Rc<dyn Fn()>);
}

/// In-memory history for tests and SSR.
#[derive(Clone, Default)]
pub struct MemoryHistory {
    inner: Rc<RefCell<MemoryInner>>,
}

#[derive(Default)]
struct MemoryInner {
    stack: Vec<String>,
    pos: usize,
    subs: Vec<Rc<dyn Fn()>>,
}

impl MemoryHistory {
    pub fn at(url: &str) -> Self {
        let h = MemoryHistory::default();
        h.inner.borrow_mut().stack = vec![url.to_string()];
        h
    }

    fn notify(&self) {
        let subs = self.inner.borrow().subs.clone();
        for cb in subs {
            cb();
        }
    }
}

impl History for MemoryHistory {
    fn current(&self) -> String {
        self.inner
            .borrow()
            .stack
            .get(self.inner.borrow().pos)
            .cloned()
            .unwrap_or_default()
    }

    fn push(&self, url: &str) {
        let mut inner = self.inner.borrow_mut();
        let pos = inner.pos;
        inner.stack.truncate(pos + 1);
        inner.stack.push(url.to_string());
        inner.pos = inner.stack.len() - 1;
        drop(inner);
        self.notify();
    }

    fn replace(&self, url: &str) {
        let mut inner = self.inner.borrow_mut();
        let pos = inner.pos;
        if pos < inner.stack.len() {
            inner.stack[pos] = url.to_string();
        } else {
            inner.stack.push(url.to_string());
            inner.pos = inner.stack.len() - 1;
        }
        drop(inner);
        self.notify();
    }

    fn back(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.pos > 0 {
            inner.pos -= 1;
        }
        drop(inner);
        self.notify();
    }

    fn subscribe(&self, cb: Rc<dyn Fn()>) {
        self.inner.borrow_mut().subs.push(cb);
    }
}

// ---------------------------------------------------------------------------
// Route definitions
// ---------------------------------------------------------------------------

enum Seg {
    Literal(String),
    Param(String),
}

struct RouteDef {
    segments: Vec<Seg>,
    target: Rc<dyn RouteTarget>,
    children: Vec<RouteDef>,
}

/// Builder for route rules.
#[derive(Default)]
pub struct Rules {
    routes: Vec<RouteDef>,
    redirects: Vec<(String, String)>,
}

impl Rules {
    pub fn new() -> Self {
        Rules::default()
    }

    /// Add a top-level route. `path` like `"/users/:id"`.
    pub fn route(mut self, path: &str, target: impl RouteTarget + 'static) -> Self {
        self.routes.push(RouteDef {
            segments: parse_segments(path),
            target: Rc::new(target),
            children: Vec::new(),
        });
        self
    }

    /// Add a redirect (resolved before matching, max 10 hops).
    pub fn redirect(mut self, from: &str, to: &str) -> Self {
        self.redirects.push((from.to_string(), to.to_string()));
        self
    }

    /// Nest child routes under `path`, rendered inside `layout`'s outlet.
    pub fn nest(
        mut self,
        path: &str,
        layout: impl RouteTarget + 'static,
        build: impl FnOnce(Rules) -> Rules,
    ) -> Self {
        let child_rules = build(Rules::new());
        self.routes.push(RouteDef {
            segments: parse_segments(path),
            target: Rc::new(layout),
            children: child_rules.routes,
        });
        self
    }
}

fn parse_segments(path: &str) -> Vec<Seg> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            if let Some(name) = s.strip_prefix(':') {
                Seg::Param(name.to_string())
            } else {
                Seg::Literal(s.to_string())
            }
        })
        .collect()
}

fn match_segments(def: &RouteDef, segments: &[&str]) -> Option<Params> {
    if def.segments.len() != segments.len() {
        return None;
    }
    let mut params = Params::default();
    for (seg, value) in def.segments.iter().zip(segments.iter()) {
        match seg {
            Seg::Literal(l) => {
                if l != value {
                    return None;
                }
            }
            Seg::Param(name) => {
                params.0.insert(name.clone(), value.to_string());
            }
        }
    }
    Some(params)
}

/// A resolved route: target + params + the child outlet VNode.
pub struct RouteMatch {
    pub target: Rc<dyn RouteTarget>,
    pub params: Params,
    pub outlet: VNode,
    /// Code-splitting chunk for the matched route (if declared).
    pub chunk: Option<ChunkInfo>,
}

impl Router {
    fn match_path(&self, path: &str) -> Option<RouteMatch> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        for def in &self.rules.routes {
            if let Some(m) = match_def(def, &segments) {
                return Some(m);
            }
        }
        None
    }
}

fn match_def(def: &RouteDef, segments: &[&str]) -> Option<RouteMatch> {
    if def.segments.len() > segments.len() {
        return None;
    }
    let params = match_segments(def, &segments[..def.segments.len()])?;
    let rest = &segments[def.segments.len()..];
    if def.children.is_empty() {
        // Leaf: must consume the whole path.
        if !rest.is_empty() {
            return None;
        }
        return Some(RouteMatch {
            target: def.target.clone(),
            params,
            outlet: VNode::Empty,
            chunk: def.target.chunk(),
        });
    }
    // Nested: try children against the rest; an empty-path child ("")
    // matches the remaining segments.
    for child in &def.children {
        if let Some(child_match) = match_def(child, rest) {
            let outlet = (child_match.target).render(&child_match.params, child_match.outlet);
            return Some(RouteMatch {
                target: def.target.clone(),
                params,
                outlet,
                chunk: child_match.chunk.clone().or_else(|| def.target.chunk()),
            });
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// The router handle: matches the current location and notifies subscribers
/// (typically `AppHandle::re_render`) on navigation.
pub struct Router {
    rules: Rc<Rules>,
    history: Rc<dyn History>,
}

impl Router {
    pub fn new(rules: Rules, history: Rc<dyn History>) -> Rc<Self> {
        Rc::new(Router {
            rules: Rc::new(rules),
            history,
        })
    }

    /// Register a change callback (called on push/replace/back, and on
    /// browser popstate via `ferrum_web::BrowserHistory`).
    pub fn on_change(&self, cb: Rc<dyn Fn()>) {
        self.history.subscribe(cb);
    }

    /// The match for the current location (follows redirects).
    pub fn current(&self) -> RouteMatch {
        let mut path = self.history.current();
        for _ in 0..10 {
            if let Some((_, to)) = self
                .rules
                .redirects
                .iter()
                .find(|(from, _)| normalize(from) == normalize(&path))
            {
                path = to.clone();
            } else {
                break;
            }
        }
        self.match_path(&normalize(&path)).unwrap_or(RouteMatch {
            target: Rc::new(NotFound),
            params: Params::default(),
            outlet: VNode::Empty,
            chunk: None,
        })
    }

    /// Navigate (push history + notify subscribers).
    pub fn navigate(&self, to: &str) {
        self.history.push(to);
    }
}

fn normalize(path: &str) -> String {
    let p = path.split(['?', '#']).next().unwrap_or(path);
    if p.is_empty() {
        "/".to_string()
    } else {
        p.to_string()
    }
}

/// Fallback target when nothing matches.
struct NotFound;

impl RouteTarget for NotFound {
    fn render(&self, _params: &Params, _outlet: VNode) -> VNode {
        ferrum_core::rsx! {
            div { class: "not-found",
                h1 { "404" }
                p { "Page not found" }
            }
        }
    }
}