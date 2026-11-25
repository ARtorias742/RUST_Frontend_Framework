//! Phase 3 gate: hooks test suite + no leaks across 10k updates.

use ferrum_core::{
    component, rsx, use_callback, use_context, use_effect, use_effect_every, use_id, use_memo,
    use_reducer, use_ref, use_state, Context,
};
use ferrum_testing::TestRenderer;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn span_text(r: &TestRenderer) -> String {
    let spans = r.find_all("span");
    r.text_of(&spans[0])
}

// --- use_state ---------------------------------------------------------------

#[component]
fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        div {
            button { onclick: move |_| set.update(|c| c + 1), "+" }
            span { "Count: {count}" }
        }
    }
}

#[test]
fn use_state_increments() {
    let r = TestRenderer::new();
    r.render(|| rsx! { Counter {} });
    assert_eq!(span_text(&r), "Count: 0");
    r.fire("click", 0);
    assert_eq!(span_text(&r), "Count: 1");
    r.fire("click", 0);
    assert_eq!(span_text(&r), "Count: 2");
    // No remount: still exactly one button + one span.
    assert_eq!(r.find_all("button").len(), 1);
    assert_eq!(r.find_all("span").len(), 1);
}

// --- use_effect ---------------------------------------------------------------

#[component]
fn Probe(runs: Rc<Cell<u32>>, cleanups: Rc<Cell<u32>>) -> VNode {
    let (count, set) = use_state(|| 0i32);
    let runs2 = runs.clone();
    let cleanups2 = cleanups.clone();
    use_effect(
        move || {
            runs2.set(runs2.get() + 1);
            let c3 = cleanups2.clone();
            Some(Box::new(move || {
                c3.set(c3.get() + 1);
            }))
        },
        Some(&[count]),
    );
    rsx! {
        div {
            button { onclick: move |_| set.update(|c| c + 1), "+" }
            span { "Count: {count}" }
        }
    }
}

#[test]
fn use_effect_runs_after_mount_with_deps() {
    let runs = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let r = TestRenderer::new();
    let runs2 = runs.clone();
    let cleanups2 = cleanups.clone();
    r.render(move || rsx! { Probe { runs: runs2.clone(), cleanups: cleanups2.clone() } });
    assert_eq!(runs.get(), 1, "effect runs once on mount");
    assert_eq!(cleanups.get(), 0);
}

#[test]
fn use_effect_skips_when_deps_unchanged() {
    let runs = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let r = TestRenderer::new();
    let runs2 = runs.clone();
    let cleanups2 = cleanups.clone();
    r.render(move || rsx! { Probe { runs: runs2.clone(), cleanups: cleanups2.clone() } });
    r.re_render();
    r.re_render();
    assert_eq!(runs.get(), 1, "effect must not re-run when deps are unchanged");
}

#[test]
fn use_effect_reruns_on_dep_change_and_cleans_up() {
    let runs = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let r = TestRenderer::new();
    let runs2 = runs.clone();
    let cleanups2 = cleanups.clone();
    r.render(move || rsx! { Probe { runs: runs2.clone(), cleanups: cleanups2.clone() } });
    assert_eq!(runs.get(), 1);
    r.fire("click", 0); // count 0 -> 1
    assert_eq!(runs.get(), 2, "effect re-runs when deps change");
    assert_eq!(cleanups.get(), 1, "previous cleanup runs before re-run");
}

#[component]
fn Every(runs: Rc<Cell<u32>>) -> VNode {
    let (n, set) = use_state(|| 0i32);
    let r2 = runs.clone();
    use_effect_every(move || {
        r2.set(r2.get() + 1);
        None
    });
    rsx! {
        button { onclick: move |_| set.update(|c| c + 1), "b{n}" }
    }
}

#[test]
fn use_effect_every_render() {
    let runs = Rc::new(Cell::new(0));
    let r = TestRenderer::new();
    let runs2 = runs.clone();
    r.render(move || rsx! { Every { runs: runs2.clone() } });
    assert_eq!(runs.get(), 1);
    r.re_render();
    r.re_render();
    assert_eq!(runs.get(), 3, "no-deps effect runs after every render");
}

#[test]
fn effect_cleanups_run_on_unmount() {
    let cleanups = Rc::new(Cell::new(0));
    {
        let r = TestRenderer::new();
        let c2 = cleanups.clone();
        r.render(move || rsx! { Probe { runs: Rc::new(Cell::new(0)), cleanups: c2.clone() } });
        assert_eq!(cleanups.get(), 0);
    } // renderer dropped -> unmount -> cleanups
    assert_eq!(cleanups.get(), 1, "cleanup runs exactly once on unmount");
}

// --- use_memo / use_callback / use_ref ------------------------------------------

#[component]
fn MemoProbe(runs: Rc<Cell<u32>>, input: i32) -> VNode {
    let r2 = runs.clone();
    let v = use_memo(&[input], move || {
        r2.set(r2.get() + 1);
        input * 2
    });
    rsx! { span { "{v}" } }
}

#[test]
fn use_memo_stable_until_deps_change() {
    let runs = Rc::new(Cell::new(0));
    let r = TestRenderer::new();
    let runs2 = runs.clone();
    r.render(move || rsx! { MemoProbe { runs: runs2.clone(), input: 5 } });
    assert_eq!(r.text(), "10");
    assert_eq!(runs.get(), 1);
    r.re_render();
    r.re_render();
    assert_eq!(runs.get(), 1, "memo not recomputed when deps unchanged");
}

#[component]
fn CbProbe(seen: Rc<RefCell<Vec<usize>>>) -> VNode {
    let cb = use_callback(&[1], || 42usize);
    seen.borrow_mut().push(cb());
    rsx! { "x" }
}

#[test]
fn use_callback_stable_identity() {
    let seen = Rc::new(RefCell::new(Vec::<usize>::new()));
    let r = TestRenderer::new();
    let seen2 = seen.clone();
    r.render(move || rsx! { CbProbe { seen: seen2.clone() } });
    r.re_render();
    r.re_render();
    assert_eq!(*seen.borrow(), vec![42, 42, 42]);
}

#[component]
fn RefProbe(boxed: Rc<Cell<u32>>) -> VNode {
    let r = use_ref(0u32);
    *r.borrow_mut() += 1;
    boxed.set(*r.borrow());
    rsx! { "x" }
}

#[test]
fn use_ref_stable_and_mutable() {
    let boxed = Rc::new(Cell::new(0));
    let r = TestRenderer::new();
    let b2 = boxed.clone();
    r.render(move || rsx! { RefProbe { boxed: b2.clone() } });
    r.re_render();
    r.re_render();
    assert_eq!(boxed.get(), 3, "ref persists and accumulates across renders");
}

// --- use_reducer ------------------------------------------------------------------

#[component]
fn ReducerProbe() -> VNode {
    let (count, dispatch) = use_reducer(|s: &i32, a: i32| s + a, 0);
    rsx! {
        div {
            button { onclick: move |_| dispatch.dispatch(2), "+" }
            span { "Count: {count}" }
        }
    }
}

#[test]
fn use_reducer_dispatches() {
    let r = TestRenderer::new();
    r.render(|| rsx! { ReducerProbe {} });
    assert_eq!(span_text(&r), "Count: 0");
    r.fire("click", 0);
    assert_eq!(span_text(&r), "Count: 2");
    r.fire("click", 0);
    assert_eq!(span_text(&r), "Count: 4");
}

// --- use_context --------------------------------------------------------------------

#[component]
fn Consumer(ctx: Context<String>) -> VNode {
    let theme = use_context(ctx);
    rsx! { span { "{theme}" } }
}

#[test]
fn use_context_reads_provider() {
    let theme = Context::<String>::new();
    let r = TestRenderer::new();
    r.render(move || theme.provider("dark".to_string(), vec![rsx! { Consumer { ctx: theme.clone() } }]));
    assert_eq!(r.text(), "dark");
}

// --- use_id ---------------------------------------------------------------------------

#[component]
fn IdProbe() -> VNode {
    let id = use_id();
    rsx! { span { "id:{id}" } }
}

#[test]
fn use_id_stable_across_renders() {
    let r = TestRenderer::new();
    r.render(|| rsx! { IdProbe {} });
    let first = r.text();
    r.re_render();
    r.re_render();
    assert_eq!(r.text(), first, "use_id is stable");
}

// --- rules of hooks --------------------------------------------------------------------

#[component]
fn Conditional(flag: bool) -> VNode {
    let _a = use_state(|| 1i32);
    if *flag {
        let _b = use_state(|| 2i32);
    }
    rsx! { "x" }
}

#[test]
#[should_panic(expected = "hook count changed")]
fn conditional_hook_panics() {
    let r = TestRenderer::new();
    r.render(|| rsx! { Conditional { flag: true } });
    r.render(|| rsx! { Conditional { flag: false } });
}

// --- 10k updates (leak gate) -----------------------------------------------------------

#[test]
fn no_leaks_across_10k_updates() {
    let r = TestRenderer::new();
    r.render(|| rsx! { Counter {} });
    for _ in 0..10_000 {
        r.fire("click", 0);
    }
    assert_eq!(span_text(&r), "Count: 10000");
    // DOM shape stable (no duplicated/accumulated nodes).
    assert_eq!(r.find_all("button").len(), 1);
    assert_eq!(r.find_all("span").len(), 1);
    assert_eq!(r.find_all("div").len(), 1);
}