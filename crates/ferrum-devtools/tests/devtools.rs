//! Phase 9 gate: DevTools inspector, state timeline, network panel, and a
//! real bug diagnosed with DevTools alone (no DOM assertions in that test).

use ferrum_core::{component, rsx, use_state, VNode};
use ferrum_data::{use_query, Client, Executor, QuerySnapshot};
use ferrum_devtools::{devtools, install, uninstall, TimelineKind};
use ferrum_testing::TestRenderer;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context as TaskContext, Poll, RawWaker, RawWakerVTable, Waker};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

#[component]
fn Leaf(label: String) -> VNode {
    rsx! { span { "{label}" } }
}

#[component]
fn Branch() -> VNode {
    rsx! {
        div {
            Leaf { label: "a" }
            Leaf { label: "b" }
        }
    }
}

#[component]
fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        button { onclick: move |_| set.update(|c| c + 1), "Count: {count}" }
    }
}

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

#[component]
fn Loader(client: Rc<Client>) -> VNode {
    match use_query(client, "users", || async { Ok::<u32, String>(7) }) {
        QuerySnapshot::Loading => rsx! { p { "loading" } },
        QuerySnapshot::Ok(n) => rsx! { p { "n={n}" } },
        QuerySnapshot::Err(e) => rsx! { p { "err={e}" } },
    }
}

// Unkeyed list: the state of item 0 leaks into the new item 0 on reorder.
#[component]
fn Item(label: String) -> VNode {
    let (picked, set) = use_state(|| false);
    rsx! {
        li {
            class: if picked { "picked" } else { "" },
            onclick: move |_| set.update(|p| !p),
            "{label}"
        }
    }
}

#[component]
fn UnkeyedList(labels: Vec<String>) -> VNode {
    let items: Vec<VNode> = labels
        .iter()
        .map(|l| rsx! { Item { label: l.clone() } })
        .collect();
    rsx! { ul { {items} } }
}

#[component]
fn KeyedList(labels: Vec<String>) -> VNode {
    let items: Vec<VNode> = labels
        .iter()
        .map(|l| rsx! { Item { key: l.clone(), label: l.clone() } })
        .collect();
    rsx! { ul { {items} } }
}

fn labels(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Inspector
// ---------------------------------------------------------------------------

#[test]
fn inspector_records_the_component_tree_with_parents_and_hooks() {
    let dt = install();
    let r = TestRenderer::new();
    r.render(|| rsx! { Branch {} });

    let tree = dt.component_tree();
    assert_eq!(tree.len(), 1);
    let branch = &tree[0];
    assert_eq!(branch.record.name, "Branch");
    assert_eq!(branch.children.len(), 2, "Branch renders two Leaf children");
    assert_eq!(branch.children[0].record.name, "Leaf");
    assert_eq!(branch.children[1].record.name, "Leaf");
    assert_eq!(branch.children[0].record.children.len(), 0);
    for leaf in &branch.children {
        assert_eq!(
            leaf.record.parent,
            Some(branch.record.id),
            "parent link points at Branch"
        );
        assert!(leaf.record.hooks.is_empty());
    }
    assert_eq!(dt.stats().mounted, 3);
    uninstall();
}

#[test]
fn inspector_tracks_renders_without_remounting() {
    let dt = install();
    let r = TestRenderer::new();
    r.render(|| rsx! { Counter {} });

    let first = dt.find_by_name("Counter");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].renders, 1);
    assert_eq!(first[0].hooks[0].kind, "state");

    r.fire("click", 0);

    let after = dt.find_by_name("Counter");
    assert_eq!(after.len(), 1, "same instance, not a remount");
    assert_eq!(after[0].id, first[0].id);
    assert_eq!(after[0].renders, 2);
    assert_eq!(after[0].mounts, 1);
    uninstall();
}

// ---------------------------------------------------------------------------
// State timeline
// ---------------------------------------------------------------------------

#[test]
fn timeline_records_state_sets_and_unmounts() {
    let dt = install();
    let r = TestRenderer::new();
    r.render(|| rsx! { Counter {} });
    r.fire("click", 0);
    r.re_render();

    let timeline = dt.timeline();
    let sets: Vec<_> = timeline
        .iter()
        .filter(|e| e.kind == TimelineKind::StateSet)
        .collect();
    assert_eq!(sets.len(), 1, "one state mutation");
    assert_eq!(sets[0].component_name, Some("Counter"));
    assert!(sets[0].detail.contains("hook #0"), "{}", sets[0].detail);
    assert!(sets[0].detail.contains("i32"), "{}", sets[0].detail);
    assert!(timeline.iter().any(|e| e.kind == TimelineKind::Mount));
    assert!(timeline.iter().any(|e| e.kind == TimelineKind::Render));

    // Sequence numbers are monotonic and timestamps non-decreasing.
    let mut last_seq = 0;
    let mut last_at = 0;
    for e in &timeline {
        assert!(e.seq > last_seq);
        assert!(e.at_ms >= last_at);
        last_seq = e.seq;
        last_at = e.at_ms;
    }
    uninstall();
}

#[test]
fn recording_can_be_paused_and_cleared() {
    let dt = install();
    let r = TestRenderer::new();
    r.render(|| rsx! { Counter {} });
    assert!(!dt.timeline().is_empty());

    dt.set_recording(false);
    r.fire("click", 0);
    r.re_render();
    assert!(
        !dt.timeline().iter().any(|e| e.kind == TimelineKind::StateSet),
        "paused recording must not capture state sets"
    );

    dt.set_recording(true);
    dt.clear();
    assert!(dt.timeline().is_empty());
    uninstall();
}

// ---------------------------------------------------------------------------
// Network panel
// ---------------------------------------------------------------------------

#[test]
fn network_panel_records_request_lifecycle() {
    let dt = install();
    let exec = QueueExecutor::default();
    let client = Client::new(exec.executor());
    let r = TestRenderer::new();
    r.render(move || rsx! { Loader { client: client.clone() } });

    // Start is recorded during render; the future is still queued.
    let start = dt.network();
    assert_eq!(start.len(), 1);
    assert_eq!(start[0].key, "users");
    assert_eq!(start[0].phase, ferrum_data::RequestPhase::Start);

    exec.drain();
    r.re_render();

    let net = dt.network();
    assert_eq!(net.len(), 2);
    assert_eq!(net[1].phase, ferrum_data::RequestPhase::Success);
    assert!(net[1].duration_ms.is_some());
    assert_eq!(dt.stats().requests, 2);
    assert_eq!(dt.stats().failed_requests, 0);

    // The response is also on the timeline.
    assert!(dt
        .timeline()
        .iter()
        .any(|e| e.kind == TimelineKind::RequestSuccess));
    uninstall();
}

// ---------------------------------------------------------------------------
// Phase 9 gate: debug a real bug with DevTools only
// ---------------------------------------------------------------------------

#[test]
fn devtools_only_diagnosis_of_unkeyed_state_leak() {
    let dt = install();
    let r = TestRenderer::new();

    // 1. Render ["a", "b"] and pick the first item.
    r.render(|| rsx! { UnkeyedList { labels: labels(&["a", "b"]) } });
    r.fire("click", 0);

    let before = dt.find_by_name("Item");
    assert_eq!(before.len(), 2);
    let first_id = before[0].id;
    assert_eq!(before[0].renders, 2, "item 0 re-rendered after the click");

    // 2. Reorder to ["b", "a"] **without keys**. No DOM inspection: the
    //    DevTools timeline is the only evidence.
    r.render(|| rsx! { UnkeyedList { labels: labels(&["b", "a"]) } });
    let timeline = dt.timeline();
    let last_mount_seq = timeline
        .iter()
        .filter(|e| e.kind == TimelineKind::Mount)
        .map(|e| e.seq)
        .max()
        .unwrap();

    // Diagnosis: no new Item mounted (no mount after the reorder) and the
    // existing instance re-rendered => its state survived the reorder. The
    // picked state now belongs to the *new* first label: a state leak.
    let reordered_mounts = timeline
        .iter()
        .filter(|e| e.kind == TimelineKind::Mount && e.seq > last_mount_seq)
        .count();
    assert_eq!(reordered_mounts, 0, "unkeyed reorder reused instances");
    let after = dt.find_by_name("Item");
    assert_eq!(after[0].id, first_id, "same instance kept its state");
    assert!(after[0].renders > before[0].renders);
    uninstall();

    // 3. Same app with keys: DevTools shows unmount+mount, i.e. fresh state.
    let dt = install();
    let r = TestRenderer::new();
    r.render(|| rsx! { KeyedList { labels: labels(&["a", "b"]) } });
    r.fire("click", 0);
    r.re_render();
    let before_keyed = dt.find_by_name("Item");
    r.render(|| rsx! { KeyedList { labels: labels(&["b", "a"]) } });

    let timeline = dt.timeline();
    let keyed_mounts = timeline
        .iter()
        .filter(|e| e.kind == TimelineKind::Mount && e.component_name == Some("Item"))
        .count();
    assert!(
        keyed_mounts > before_keyed.len(),
        "keyed reorder mounts the moved item fresh (state reset): {keyed_mounts} mounts"
    );
    assert!(
        timeline.iter().any(|e| e.kind == TimelineKind::Unmount),
        "keyed reorder unmounts the removed instance"
    );
    uninstall();
}

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

#[test]
fn overlay_renders_the_snapshot() {
    let _ = install();
    let r = TestRenderer::new();
    r.render(|| rsx! { Counter {} });
    r.fire("click", 0);
    r.re_render();

    let html = {
        let overlay = ferrum_devtools::overlay();
        ferrum_ssr::render_to_string(&overlay)
    };
    assert!(html.contains("Ferrum DevTools"), "{html}");
    assert!(html.contains("Counter"), "{html}");
    assert!(html.contains("setState"), "{html}");
    uninstall();
}

#[test]
fn devtools_handle_is_stable_and_uninstall_clears_observers() {
    let a = install();
    let b = install();
    assert!(Rc::ptr_eq(&a, &b));
    assert!(devtools().is_some());
    uninstall();
    assert!(devtools().is_none());
}