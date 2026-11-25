//! Phase 4 gate (data): use_query loading -> ok, cache dedup, error, invalidate.

use ferrum_core::{component, rsx};
use ferrum_data::{use_query, Client, Executor, QuerySnapshot};
use ferrum_testing::TestRenderer;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};

/// A queueing executor: futures run when the test drains them (never inline
/// during render — mirrors `spawn_local` deferral).
type PinnedFuture = Pin<Box<dyn Future<Output = ()>>>;

#[derive(Clone, Default)]
struct QueueExecutor {
    queue: Rc<RefCell<VecDeque<PinnedFuture>>>,
}

impl QueueExecutor {
    fn executor(&self) -> Executor {
        let q = self.queue.clone();
        Rc::new(move |fut: PinnedFuture| q.borrow_mut().push_back(fut))
    }

    /// Run all queued futures to completion (may enqueue more).
    fn drain(&self) {
        while let Some(mut fut) = self.queue.borrow_mut().pop_front() {
            block_on(&mut fut);
        }
    }
}

fn block_on<F: Future + ?Sized>(fut: &mut Pin<Box<F>>) -> F::Output {
    fn noop_raw_waker() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            noop_raw_waker()
        }
        fn noop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        RawWaker::new(std::ptr::null(), &VTABLE)
    }
    let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
    let mut cx = TaskContext::from_waker(&waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => continue,
        }
    }
}

#[derive(Clone)]
struct User {
    #[allow(dead_code)]
    id: u32,
    name: String,
}

async fn fetch_users() -> Result<Vec<User>, String> {
    Ok(vec![User { id: 1, name: "Ada".into() }, User { id: 2, name: "Bob".into() }])
}

#[component]
fn UserList(client: Rc<Client>) -> VNode {
    match use_query(client, "users", fetch_users) {
        QuerySnapshot::Loading => rsx! { p { "Loading..." } },
        QuerySnapshot::Ok(users) => {
            let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();
            rsx! { ul { for n in names { li { "{n}" } } } }
        }
        QuerySnapshot::Err(e) => rsx! { p { "Error: {e}" } },
    }
}

#[test]
fn use_query_loading_then_ok() {
    let exec = QueueExecutor::default();
    let client = Client::new(exec.executor());
    let r = TestRenderer::new();
    let c2 = client.clone();
    r.render(move || rsx! { UserList { client: c2.clone() } });
    assert_eq!(r.text(), "Loading...");
    exec.drain();
    assert_eq!(r.text(), "AdaBob");
}

#[test]
fn use_query_cache_dedupes_inflight() {
    let exec = QueueExecutor::default();
    let client = Client::new(exec.executor());
    let r = TestRenderer::new();
    let c2 = client.clone();
    r.render(move || rsx! { UserList { client: c2.clone() } });
    // Re-render while in flight: must NOT spawn a second fetch.
    r.re_render();
    r.re_render();
    exec.drain();
    assert_eq!(r.text(), "AdaBob");
    // Drain again: nothing new (no refetch loop).
    exec.drain();
    assert_eq!(r.text(), "AdaBob");
}

#[component]
fn Failing() -> VNode {
    let client = use_context_client();
    match use_query(&client, "boom", || async { Err::<u32, _>("boom".to_string()) }) {
        QuerySnapshot::Loading => rsx! { p { "Loading..." } },
        QuerySnapshot::Ok(_) => rsx! { p { "ok" } },
        QuerySnapshot::Err(e) => rsx! { p { "Error: {e}" } },
    }
}

// Helper: fetch the client from a thread-local set by the test (avoids
// plumbing it through every probe).
fn use_context_client() -> Rc<Client> {
    CLIENT.with(|c| c.borrow().clone().expect("client not set"))
}

thread_local! {
    static CLIENT: RefCell<Option<Rc<Client>>> = const { RefCell::new(None) };
}

#[test]
fn use_query_error_state() {
    let exec = QueueExecutor::default();
    let client = Client::new(exec.executor());
    CLIENT.with(|c| *c.borrow_mut() = Some(client));
    let r = TestRenderer::new();
    r.render(|| rsx! { Failing {} });
    assert_eq!(r.text(), "Loading...");
    exec.drain();
    assert_eq!(r.text(), "Error: boom");
}

#[test]
fn invalidate_clears_cache() {
    let exec = QueueExecutor::default();
    let client = Client::new(exec.executor());
    let r = TestRenderer::new();
    let c2 = client.clone();
    r.render(move || rsx! { UserList { client: c2.clone() } });
    exec.drain();
    assert_eq!(r.text(), "AdaBob");
    client.invalidate("users");
    // Remount (fresh component instance) refetches.
    let r2 = TestRenderer::new();
    let c3 = client.clone();
    r2.render(move || rsx! { UserList { client: c3.clone() } });
    assert_eq!(r2.text(), "Loading...");
    exec.drain();
    assert_eq!(r2.text(), "AdaBob");
}