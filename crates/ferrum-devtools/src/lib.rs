//! # ferrum-devtools
//!
//! The Ferrum DevTools layer: a **component inspector**, a **state
//! timeline**, and a **network panel**, plus an in-app overlay.
//!
//! It is observation-only and dependency-injected through the existing
//! extension points, so it never changes renderer behaviour:
//!
//! - the reconciler reports component mount/render/unmount through
//!   [`ferrum_reconciler::Observer`],
//! - `ferrum-core` reports `set`/`update`/`dispatch` through
//!   [`ferrum_core::HookObserver`],
//! - `ferrum-data` reports request start/success/error through
//!   [`ferrum_data::RequestObserver`].
//!
//! Install once at startup (browser or headless):
//!
//! ```ignore
//! ferrum_devtools::install();
//! ```
//!
//! Then read a snapshot at any time (`devtools().snapshot()`), or mount
//! [`overlay`] inside the app tree to get an in-page inspector.
//!
//! Everything is thread-local and cheap: with no observer installed the
//! reconciler pays one `Option` check per component operation.

use ferrum_core::{HookEvent, HookInfo, VNode};
use ferrum_data::{RequestPhase, RequestRecord};
use ferrum_reconciler::Observer;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

/// A component instance in the inspector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRecord {
    pub id: u64,
    pub name: &'static str,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
    pub hooks: Vec<HookInfo>,
    pub mounts: u32,
    pub renders: u32,
    pub mounted: bool,
}

/// One entry in the state timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineKind {
    Mount,
    Render,
    Unmount,
    StateSet,
    ReducerDispatch,
    RequestStart,
    RequestSuccess,
    RequestError,
}

impl TimelineKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimelineKind::Mount => "mount",
            TimelineKind::Render => "render",
            TimelineKind::Unmount => "unmount",
            TimelineKind::StateSet => "setState",
            TimelineKind::ReducerDispatch => "dispatch",
            TimelineKind::RequestStart => "request",
            TimelineKind::RequestSuccess => "response",
            TimelineKind::RequestError => "error",
        }
    }
}

/// A timestamped timeline entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEntry {
    pub seq: u64,
    pub at_ms: u128,
    pub kind: TimelineKind,
    pub component: Option<u64>,
    pub component_name: Option<&'static str>,
    pub detail: String,
}

/// A network entry (one per request phase).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkEntry {
    pub key: String,
    pub phase: RequestPhase,
    pub at_ms: u128,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// A point-in-time copy of the whole DevTools state.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub elapsed_ms: u128,
    pub components: Vec<ComponentRecord>,
    pub timeline: Vec<TimelineEntry>,
    pub network: Vec<NetworkEntry>,
}

/// A nested inspector node.
#[derive(Debug, Clone)]
pub struct ComponentNode {
    pub record: ComponentRecord,
    pub children: Vec<ComponentNode>,
}

/// Aggregate counters for the status bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DevToolsStats {
    pub components: usize,
    pub mounted: usize,
    pub timeline: usize,
    pub network: usize,
    pub requests: usize,
    pub failed_requests: usize,
}

// ---------------------------------------------------------------------------
// DevTools
// ---------------------------------------------------------------------------

struct Inner {
    started: Instant,
    seq: u64,
    recording: bool,
    components: Vec<ComponentRecord>,
    by_id: HashMap<u64, usize>,
    timeline: VecDeque<TimelineEntry>,
    network: VecDeque<NetworkEntry>,
    max_timeline: usize,
    max_network: usize,
    highlight: Option<u64>,
    requests: u64,
    failed_requests: u64,
}

/// The DevTools store. Cheap to clone (`Rc` inside); one per thread.
#[derive(Clone)]
pub struct DevTools(Rc<RefCell<Inner>>);

impl DevTools {
    pub fn new() -> Self {
        DevTools(Rc::new(RefCell::new(Inner {
            started: Instant::now(),
            seq: 0,
            recording: true,
            components: Vec::new(),
            by_id: HashMap::new(),
            timeline: VecDeque::new(),
            network: VecDeque::new(),
            max_timeline: 2000,
            max_network: 500,
            highlight: None,
            requests: 0,
            failed_requests: 0,
        })))
    }

    // -- recording -----------------------------------------------------------

    fn push_timeline(
        &self,
        kind: TimelineKind,
        component: Option<u64>,
        name: Option<&'static str>,
        detail: String,
    ) {
        let mut inner = self.0.borrow_mut();
        if !inner.recording {
            return;
        }
        inner.seq += 1;
        let seq = inner.seq;
        let at_ms = inner.started.elapsed().as_millis();
        inner.timeline.push_back(TimelineEntry {
            seq,
            at_ms,
            kind,
            component,
            component_name: name,
            detail,
        });
        while inner.timeline.len() > inner.max_timeline {
            inner.timeline.pop_front();
        }
    }

    pub(crate) fn record_mount(
        &self,
        id: u64,
        name: &'static str,
        parent: Option<u64>,
        hooks: &[HookInfo],
    ) {
        {
            let mut inner = self.0.borrow_mut();
            if !inner.recording {
                return;
            }
            let record = ComponentRecord {
                id,
                name,
                parent,
                children: Vec::new(),
                hooks: hooks.to_vec(),
                mounts: 1,
                renders: 1,
                mounted: true,
            };
            let slot = inner.components.len();
            inner.by_id.insert(id, slot);
            inner.components.push(record);
            if let Some(p) = parent {
                if let Some(&pi) = inner.by_id.get(&p) {
                    inner.components[pi].children.push(id);
                }
            }
        }
        self.push_timeline(
            TimelineKind::Mount,
            Some(id),
            Some(name),
            format!("{} mounted", name),
        );
    }

    pub(crate) fn record_render(&self, id: u64, name: &'static str, hooks: &[HookInfo]) {
        {
            let mut inner = self.0.borrow_mut();
            if !inner.recording {
                return;
            }
            if let Some(&i) = inner.by_id.get(&id) {
                inner.components[i].renders += 1;
                inner.components[i].hooks = hooks.to_vec();
            }
        }
        self.push_timeline(
            TimelineKind::Render,
            Some(id),
            Some(name),
            format!("{} rendered", name),
        );
    }

    pub(crate) fn record_unmount(&self, id: u64, name: &'static str) {
        {
            let mut inner = self.0.borrow_mut();
            if !inner.recording {
                return;
            }
            if let Some(&i) = inner.by_id.get(&id) {
                inner.components[i].mounted = false;
            }
        }
        self.push_timeline(
            TimelineKind::Unmount,
            Some(id),
            Some(name),
            format!("{} unmounted", name),
        );
    }

    pub(crate) fn record_hook(&self, event: HookEvent) {
        match event {
            HookEvent::StateSet {
                instance,
                name,
                slot,
                value,
            } => {
                self.push_timeline(
                    TimelineKind::StateSet,
                    Some(instance),
                    Some(name),
                    format!("{} set hook #{} -> {}", name, slot, value),
                );
            }
            HookEvent::ReducerDispatch {
                instance,
                name,
                slot,
            } => {
                self.push_timeline(
                    TimelineKind::ReducerDispatch,
                    Some(instance),
                    Some(name),
                    format!("{} dispatched hook #{}", name, slot),
                );
            }
        }
    }

    pub(crate) fn record_request(&self, record: RequestRecord) {
        {
            let mut inner = self.0.borrow_mut();
            if !inner.recording {
                return;
            }
            inner.requests += 1;
            if record.phase == RequestPhase::Error {
                inner.failed_requests += 1;
            }
            inner.network.push_back(NetworkEntry {
                key: record.key.clone(),
                phase: record.phase,
                at_ms: record.at_ms,
                duration_ms: record.duration_ms,
                error: record.error.clone(),
            });
            while inner.network.len() > inner.max_network {
                inner.network.pop_front();
            }
        }
        let (kind, detail) = match record.phase {
            RequestPhase::Start => (TimelineKind::RequestStart, format!("GET {}", record.key)),
            RequestPhase::Success => (
                TimelineKind::RequestSuccess,
                format!(
                    "GET {} -> 200 ({}ms)",
                    record.key,
                    record.duration_ms.unwrap_or(0)
                ),
            ),
            RequestPhase::Error => (
                TimelineKind::RequestError,
                format!("GET {} -> {}", record.key, record.error.unwrap_or_default()),
            ),
        };
        self.push_timeline(kind, None, None, detail);
    }

    // -- controls ------------------------------------------------------------

    /// Pause/resume recording (the overlay's record button).
    pub fn set_recording(&self, recording: bool) {
        self.0.borrow_mut().recording = recording;
    }

    pub fn recording(&self) -> bool {
        self.0.borrow().recording
    }

    /// Highlight a component id in the inspector.
    pub fn set_highlight(&self, id: Option<u64>) {
        self.0.borrow_mut().highlight = id;
    }

    pub fn highlight(&self) -> Option<u64> {
        self.0.borrow().highlight
    }

    /// Drop all recorded data (keeps the component registry).
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        inner.timeline.clear();
        inner.network.clear();
        inner.requests = 0;
        inner.failed_requests = 0;
    }

    /// Full reset (new store contents, same handle).
    pub fn reset(&self) {
        let mut inner = self.0.borrow_mut();
        *inner = Inner {
            started: Instant::now(),
            seq: 0,
            recording: true,
            components: Vec::new(),
            by_id: HashMap::new(),
            timeline: VecDeque::new(),
            network: VecDeque::new(),
            max_timeline: 2000,
            max_network: 500,
            highlight: None,
            requests: 0,
            failed_requests: 0,
        };
    }

    // -- queries -------------------------------------------------------------

    pub fn snapshot(&self) -> Snapshot {
        let inner = self.0.borrow();
        Snapshot {
            elapsed_ms: inner.started.elapsed().as_millis(),
            components: inner.components.clone(),
            timeline: inner.timeline.iter().cloned().collect(),
            network: inner.network.iter().cloned().collect(),
        }
    }

    pub fn components(&self) -> Vec<ComponentRecord> {
        self.0.borrow().components.clone()
    }

    /// Components with `mounted == true`, in mount order.
    pub fn live_components(&self) -> Vec<ComponentRecord> {
        self.0
            .borrow()
            .components
            .iter()
            .filter(|c| c.mounted)
            .cloned()
            .collect()
    }

    pub fn find_by_name(&self, name: &str) -> Vec<ComponentRecord> {
        self.0
            .borrow()
            .components
            .iter()
            .filter(|c| c.name == name)
            .cloned()
            .collect()
    }

    pub fn timeline(&self) -> Vec<TimelineEntry> {
        self.0.borrow().timeline.iter().cloned().collect()
    }

    pub fn network(&self) -> Vec<NetworkEntry> {
        self.0.borrow().network.iter().cloned().collect()
    }

    pub fn stats(&self) -> DevToolsStats {
        let inner = self.0.borrow();
        DevToolsStats {
            components: inner.components.len(),
            mounted: inner.components.iter().filter(|c| c.mounted).count(),
            timeline: inner.timeline.len(),
            network: inner.network.len(),
            requests: inner.requests as usize,
            failed_requests: inner.failed_requests as usize,
        }
    }

    /// The live component tree, nested by `parent` links.
    pub fn component_tree(&self) -> Vec<ComponentNode> {
        let inner = self.0.borrow();
        let mut children_of: HashMap<u64, Vec<u64>> = HashMap::new();
        let mut roots = Vec::new();
        for record in &inner.components {
            match record.parent {
                Some(p) if inner.by_id.contains_key(&p) => {
                    children_of.entry(p).or_default().push(record.id);
                }
                _ => roots.push(record.id),
            }
        }
        roots
            .into_iter()
            .map(|id| build_node(id, &inner.components, &children_of))
            .collect()
    }
}

fn build_node(
    id: u64,
    components: &[ComponentRecord],
    children_of: &HashMap<u64, Vec<u64>>,
) -> ComponentNode {
    let record = components
        .iter()
        .find(|c| c.id == id)
        .expect("component record exists")
        .clone();
    let children = children_of
        .get(&id)
        .map(|ids| {
            ids.iter()
                .map(|child| build_node(*child, components, children_of))
                .collect()
        })
        .unwrap_or_default();
    ComponentNode { record, children }
}

impl Default for DevTools {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Global install
// ---------------------------------------------------------------------------

thread_local! {
    static DEVTOOLS: RefCell<Option<Rc<DevTools>>> = const { RefCell::new(None) };
}

struct ReconcilerBridge(Rc<DevTools>);

impl Observer for ReconcilerBridge {
    fn component_mount(
        &self,
        id: u64,
        name: &'static str,
        parent: Option<u64>,
        hooks: &[HookInfo],
    ) {
        self.0.record_mount(id, name, parent, hooks);
    }
    fn component_render(&self, id: u64, name: &'static str, hooks: &[HookInfo]) {
        self.0.record_render(id, name, hooks);
    }
    fn component_unmount(&self, id: u64, name: &'static str) {
        self.0.record_unmount(id, name);
    }
}

struct NetworkBridge(Rc<DevTools>);

impl ferrum_data::RequestObserver for NetworkBridge {
    fn request(&self, record: RequestRecord) {
        self.0.record_request(record);
    }
}

/// Install DevTools for the current thread and return the store.
///
/// Idempotent: calling it twice returns the same store. Call once at app
/// startup (before the first render) to capture the initial mount.
pub fn install() -> Rc<DevTools> {
    DEVTOOLS.with(|slot| {
        if let Some(existing) = slot.borrow().as_ref() {
            return existing.clone();
        }
        let devtools = Rc::new(DevTools::new());
        ferrum_reconciler::set_observer(Some(Rc::new(ReconcilerBridge(devtools.clone()))));
        let hook = devtools.clone();
        ferrum_core::set_hook_observer(Some(Rc::new(move |event: HookEvent| {
            hook.record_hook(event);
        })));
        ferrum_data::set_request_observer(Some(Rc::new(NetworkBridge(devtools.clone()))));
        *slot.borrow_mut() = Some(devtools.clone());
        devtools
    })
}

/// The installed store, if [`install`] has been called.
pub fn devtools() -> Option<Rc<DevTools>> {
    DEVTOOLS.with(|slot| slot.borrow().clone())
}

/// Remove all observers and forget the store.
pub fn uninstall() {
    ferrum_reconciler::set_observer(None);
    ferrum_core::set_hook_observer(None);
    ferrum_data::set_request_observer(None);
    DEVTOOLS.with(|slot| *slot.borrow_mut() = None);
}

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

/// An in-app DevTools panel (fixed-position, pointer-events enabled).
///
/// Mount it once, usually behind a debug flag:
///
/// ```ignore
/// rsx! { div { App {} { ferrum_devtools::overlay() } } }
/// ```
///
/// The panel renders the current snapshot on every app render; it is a
/// snapshot view, not a live-updating widget (the app's own re-renders drive
/// it).
pub fn overlay() -> VNode {
    let Some(devtools) = devtools() else {
        return VNode::Empty;
    };
    let snapshot = devtools.snapshot();

    let panel = "position:fixed;right:12px;bottom:12px;width:380px;max-height:50vh;overflow:auto;\
                 background:#0b1020;color:#e5e7eb;font:12px/1.5 ui-monospace,monospace;\
                 border:1px solid #374151;border-radius:8px;padding:8px;z-index:2147483647;";
    let head = "font-weight:700;color:#93c5fd;margin:0 0 4px 0;";
    let dim = "color:#9ca3af;";
    let mut rows: Vec<VNode> = Vec::new();

    rows.push(ferrum_core::rsx! { div { style: head, "Ferrum DevTools" } });
    rows.push(ferrum_core::rsx! {
        div { style: dim,
            "{snapshot.components.len()} components · {snapshot.timeline.len()} events · {snapshot.network.len()} requests"
        }
    });

    for node in devtools.component_tree() {
        push_tree_row(&mut rows, &node, 0);
    }

    if !snapshot.network.is_empty() {
        rows.push(ferrum_core::rsx! { div { style: head, "Network" } });
        for entry in snapshot.network.iter().rev().take(8) {
            let text = match entry.phase {
                RequestPhase::Start => format!("→ GET {}", entry.key),
                RequestPhase::Success => {
                    format!("✓ GET {} {}ms", entry.key, entry.duration_ms.unwrap_or(0))
                }
                RequestPhase::Error => {
                    format!(
                        "✗ GET {} {}",
                        entry.key,
                        entry.error.clone().unwrap_or_default()
                    )
                }
            };
            rows.push(ferrum_core::rsx! { div { style: dim, "{text}" } });
        }
    }

    if !snapshot.timeline.is_empty() {
        rows.push(ferrum_core::rsx! { div { style: head, "Timeline" } });
        for entry in snapshot.timeline.iter().rev().take(10) {
            let text = format!(
                "{:>5}ms  {:<10} {}",
                entry.at_ms,
                entry.kind.as_str(),
                entry.detail
            );
            rows.push(ferrum_core::rsx! { div { style: dim, "{text}" } });
        }
    }

    ferrum_core::rsx! { div { style: panel, {rows} } }
}

fn push_tree_row(rows: &mut Vec<VNode>, node: &ComponentNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let hooks = node
        .record
        .hooks
        .iter()
        .map(|h| format!("{}:{}", h.kind, h.index))
        .collect::<Vec<_>>()
        .join(",");
    let text = format!(
        "{}{} #{}  renders={}{}",
        indent,
        node.record.name,
        node.record.id,
        node.record.renders,
        if hooks.is_empty() {
            String::new()
        } else {
            format!("  [{}]", hooks)
        }
    );
    rows.push(ferrum_core::rsx! { div { "{text}" } });
    for child in &node.children {
        push_tree_row(rows, child, depth + 1);
    }
}
