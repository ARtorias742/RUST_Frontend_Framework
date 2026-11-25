//! Phase 7 gate: code-splitting boundary (fallback -> loaded chunk).

use ferrum_core::{rsx, VNode};
use ferrum_lazy::{Lazy, LazyExecutor, LazyFuture, LazyTask};
use ferrum_testing::TestRenderer;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn immediate_executor() -> LazyExecutor {
    Rc::new(|task: LazyTask| {
        block_on(task);
    })
}

#[test]
fn renders_fallback_until_chunk_resolves() {
    let loader: Rc<dyn Fn() -> LazyFuture> = Rc::new(|| {
        Box::pin(async {
            rsx! { section { class: "chunk", "loaded" } }
        })
    });
    let r = TestRenderer::new();
    r.render(move || {
        rsx! {
            Lazy {
                loader: loader.clone(),
                fallback: rsx! { div { class: "fallback", "loading" } },
                executor: immediate_executor(),
            }
        }
    });

    // The effect runs after mount, resolves immediately, and re-renders.
    assert!(r.html().contains("loaded"), "{}", r.html());
    assert!(!r.html().contains("fallback"), "{}", r.html());
}

#[test]
fn pending_chunk_keeps_fallback() {
    let loader: Rc<dyn Fn() -> LazyFuture> = Rc::new(|| {
        Box::pin(async {
            rsx! { "never" }
        })
    });
    let executor: LazyExecutor = Rc::new(|_task: LazyTask| {
        // Drop the task: simulates a chunk that has not arrived yet.
    });
    let r = TestRenderer::new();
    r.render(move || {
        rsx! {
            Lazy {
                loader: loader.clone(),
                fallback: rsx! { div { "loading" } },
                executor: executor.clone(),
            }
        }
    });
    assert!(r.html().contains("loading"), "{}", r.html());
    assert!(!r.html().contains("never"), "{}", r.html());
}

#[test]
fn lazy_output_is_a_normal_vnode_tree() {
    let loader: Rc<dyn Fn() -> LazyFuture> = Rc::new(|| {
        Box::pin(async {
            rsx! { ul { li { "a" } li { "b" } } }
        })
    });
    let r = TestRenderer::new();
    r.render(move || {
        rsx! {
            Lazy {
                loader: loader.clone(),
                fallback: VNode::Empty,
                executor: immediate_executor(),
            }
        }
    });
    assert_eq!(r.find_all("li").len(), 2);
}
