//! # ferrum-data
//!
//! A small data layer: a [`Client`] with a query cache + in-flight
//! deduplication, and the [`use_query`] hook that subscribes a component to
//! a query and re-renders it when the state changes.
//!
//! The executor is pluggable: `ferrum-web` apps pass
//! `wasm_bindgen_futures::spawn_local`; tests pass a queueing executor.
//! Executors **must defer** the future (run it after the current render),
//! never run it inline.
//!
//! ```ignore
//! let client = Client::new(executor);
//! // in a component:
//! let q = use_query(&client, "users", || fetch_users());
//! match q { QuerySnapshot::Loading => ..., QuerySnapshot::Ok(v) => ..., QuerySnapshot::Err(e) => ... }
//! ```

use ferrum_core::{current_re_render, use_custom_rc};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// A boxed, `'static` future the executor will run.
pub type BoxFuture = Pin<Box<dyn Future<Output = ()>>>;
/// Runs a future (e.g. `spawn_local`, or a test queue).
pub type Executor = Rc<dyn Fn(BoxFuture)>;

// ---------------------------------------------------------------------------
// Network observation (DevTools)
// ---------------------------------------------------------------------------

/// Lifecycle phase of an observed request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPhase {
    Start,
    Success,
    Error,
}

/// One network entry reported to the DevTools network panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestRecord {
    pub key: String,
    pub phase: RequestPhase,
    /// Milliseconds since the observer was installed.
    pub at_ms: u128,
    /// Wall time of the request, set on `Success`/`Error`.
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Observer installed by `ferrum-devtools` (thread-local, no-op by default).
pub trait RequestObserver {
    fn request(&self, record: RequestRecord);
}

type RequestObserverRef = Rc<dyn RequestObserver>;

thread_local! {
    static REQUEST_OBSERVER: RefCell<Option<RequestObserverRef>> = const { RefCell::new(None) };
    static REQUEST_EPOCH: Cell<u128> = const { Cell::new(0) };
    static REQUEST_CLOCK: Cell<u128> = const { Cell::new(0) };
}

fn tick() -> u128 {
    REQUEST_CLOCK.with(|c| {
        let next = c.get() + 1;
        c.set(next);
        next
    })
}

/// Install (or clear with `None`) the request observer.
pub fn set_request_observer(observer: Option<RequestObserverRef>) {
    REQUEST_OBSERVER.with(|o| *o.borrow_mut() = observer);
    REQUEST_EPOCH.with(|e| e.set(tick()));
}

fn emit_request(key: &str, phase: RequestPhase, started: u128, error: Option<String>) {
    REQUEST_OBSERVER.with(|o| {
        let observer = o.borrow().clone();
        if let Some(observer) = observer {
            let at_ms = tick().saturating_sub(REQUEST_EPOCH.with(|e| e.get()));
            observer.request(RequestRecord {
                key: key.to_string(),
                phase,
                at_ms,
                duration_ms: (phase != RequestPhase::Start)
                    .then(|| tick().saturating_sub(started) as u64),
                error,
            });
        }
    });
}

/// The state of a cached query.
enum QueryState {
    Loading,
    Ok(Rc<dyn Any>),
    Err(String),
}

type Watchers = RefCell<Vec<(u64, Rc<dyn Fn()>)>>;

struct QueryEntry {
    state: RefCell<QueryState>,
    watchers: Watchers,
    inflight: Cell<bool>,
}

impl QueryEntry {
    fn notify(&self) {
        for (_, cb) in self.watchers.borrow().iter() {
            cb();
        }
    }

    fn register(&self, id: u64, cb: Rc<dyn Fn()>) {
        self.watchers.borrow_mut().push((id, cb));
    }

    fn unregister(&self, id: u64) {
        self.watchers.borrow_mut().retain(|(i, _)| *i != id);
    }

    /// Owned snapshot of the current state (no borrows escape).
    fn snapshot<T: Clone + 'static>(&self) -> QuerySnapshot<T> {
        let state = self.state.borrow();
        match &*state {
            QueryState::Loading => QuerySnapshot::Loading,
            QueryState::Ok(v) => QuerySnapshot::Ok(
                v.downcast_ref::<T>()
                    .expect("use_query: type changed for key")
                    .clone(),
            ),
            QueryState::Err(e) => QuerySnapshot::Err(e.clone()),
        }
    }
}

/// The query cache + executor.
pub struct Client {
    cache: RefCell<HashMap<String, Rc<QueryEntry>>>,
    executor: Executor,
}

impl Client {
    pub fn new(executor: Executor) -> Rc<Self> {
        Rc::new(Client {
            cache: RefCell::new(HashMap::new()),
            executor,
        })
    }

    /// Drop a cached query (it will refetch on next use).
    pub fn invalidate(&self, key: &str) {
        self.cache.borrow_mut().remove(key);
    }

    fn entry_for(&self, key: &str) -> Rc<QueryEntry> {
        if let Some(e) = self.cache.borrow().get(key) {
            return e.clone();
        }
        let entry = Rc::new(QueryEntry {
            state: RefCell::new(QueryState::Loading),
            watchers: RefCell::new(Vec::new()),
            inflight: Cell::new(false),
        });
        self.cache
            .borrow_mut()
            .insert(key.to_string(), entry.clone());
        entry
    }
}

/// A snapshot of a query for the current render.
#[derive(Clone)]
pub enum QuerySnapshot<T> {
    Loading,
    Ok(T),
    Err(String),
}

/// Per-component query subscription (stored in a custom hook slot; the Drop
/// impl unregisters the watcher on unmount).
struct QuerySlot {
    #[allow(dead_code)]
    key: String,
    entry: Rc<QueryEntry>,
    watcher: u64,
}

impl Drop for QuerySlot {
    fn drop(&mut self) {
        self.entry.unregister(self.watcher);
    }
}

/// Subscribe to `key`: register a watcher and start the fetch if the entry
/// is fresh and idle.
fn subscribe<T, F, Fut>(
    client: &Client,
    key: &str,
    fetch: &F,
    re_render: &Rc<dyn Fn()>,
) -> QuerySlot
where
    T: Clone + 'static,
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, String>> + 'static,
{
    let entry = client.entry_for(key);
    let id = next_watcher_id();
    entry.register(id, re_render.clone());
    let need_spawn = matches!(*entry.state.borrow(), QueryState::Loading) && !entry.inflight.get();
    if need_spawn {
        entry.inflight.set(true);
        let entry2 = entry.clone();
        let fut = fetch();
        let started = tick();
        let key = key.to_string();
        emit_request(&key, RequestPhase::Start, started, None);
        (client.executor)(Box::pin(async move {
            match fut.await {
                Ok(v) => {
                    *entry2.state.borrow_mut() = QueryState::Ok(Rc::new(v));
                    entry2.inflight.set(false);
                    emit_request(&key, RequestPhase::Success, started, None);
                    entry2.notify();
                }
                Err(e) => {
                    *entry2.state.borrow_mut() = QueryState::Err(e.clone());
                    entry2.inflight.set(false);
                    emit_request(&key, RequestPhase::Error, started, Some(e));
                    entry2.notify();
                }
            }
        }));
    }
    QuerySlot {
        key: key.to_string(),
        entry,
        watcher: id,
    }
}

/// `useQuery`: subscribe to `key`, fetching with `fetch` on first use.
/// Re-renders the component when the query settles.
pub fn use_query<T, F, Fut>(client: &Client, key: &str, fetch: F) -> QuerySnapshot<T>
where
    T: Clone + 'static,
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<T, String>> + 'static,
{
    let re_render = current_re_render();
    let slot = use_custom_rc(|| -> QuerySlot { subscribe(client, key, &fetch, &re_render) });

    // Key changed since last render: replace the slot (Drop unregisters the
    // old watcher) and subscribe to the new entry.
    {
        let mut slot_ref = slot.borrow_mut();
        if slot_ref.key != key {
            *slot_ref = subscribe(client, key, &fetch, &re_render);
        }
    }

    let state = slot.borrow();
    let snapshot = state.entry.snapshot::<T>();
    drop(state);
    snapshot
}

fn next_watcher_id() -> u64 {
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
