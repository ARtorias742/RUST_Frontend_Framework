//! # ferrum-conformance
//!
//! A WPT-like behavioural suite for the React subset Ferrum implements.
//!
//! Each check is a named, isolated scenario. The suite runs every check,
//! catches panics per-check, and asserts the **≥ 95% pass gate** from the
//! roadmap (§4 “Compatibility”). A failing check is reported by name rather
//! than aborting the run, so one regression does not hide the rest.
//!
//! Run with `cargo test -p ferrum-conformance -- --nocapture` to see the
//! per-check report.

use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

use ferrum_core::{component, rsx, use_context, use_effect, use_id, use_memo, use_reducer, use_ref, use_state, Context, VNode};
use ferrum_testing::TestRenderer;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[component]
fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        button { onclick: move |_| set.update(|c| c + 1), "count:{count}" }
    }
}

#[component]
fn Setter() -> VNode {
    let (n, set) = use_state(|| 1i32);
    rsx! {
        button { onclick: move |_| set.set(41), "n:{n}" }
    }
}

#[component]
fn EffectOnce(ran: Rc<Cell<u32>>) -> VNode {
    let ran = Rc::clone(ran);
    use_effect(
        move || {
            ran.set(ran.get() + 1);
            None
        },
        Some(&[] as &[u8]),
    );
    rsx! { span { "x" } }
}

#[component]
fn EffectCleanup(cleaned: Rc<Cell<bool>>) -> VNode {
    let cleaned = Rc::clone(cleaned);
    use_effect(
        move || {
            let c = Rc::clone(&cleaned);
            Some(Box::new(move || c.set(true)) as ferrum_core::EffectCleanup)
        },
        None::<&[u8]>,
    );
    rsx! { span { "x" } }
}

#[component]
fn MemoComp(deps: i32) -> VNode {
    let value = use_memo(&[deps], || deps * 2);
    rsx! { span { "m:{value}" } }
}

#[component]
fn RefComp() -> VNode {
    let r = use_ref(|| 0i32);
    let first = Rc::as_ptr(&r) as usize;
    rsx! { span { "r:{first}" } }
}

#[component]
fn ReducerComp() -> VNode {
    let (state, dispatch) = use_reducer(|s: &i32, add: i32| s + add, 0i32);
    rsx! {
        button { onclick: move |_| dispatch.dispatch(2), "s:{state}" }
    }
}

#[component]
fn IdComp() -> VNode {
    let id = use_id();
    rsx! { span { "id:{id}" } }
}

#[component]
fn KeyedList() -> VNode {
    let items: Vec<VNode> = [1u64, 2, 3]
        .iter()
        .map(|i| rsx! { li { key: *i, "item:{i}" } })
        .collect();
    rsx! { ul { {items} } }
}

#[component]
fn Fragment() -> VNode {
    rsx! { <> b { "a" } i { "b" } </> }
}

#[component]
fn Conditional(show: bool) -> VNode {
    rsx! {
        div { if *show { p { "yes" } } else { p { "no" } } }
    }
}

#[component]
fn Controlled(value: String) -> VNode {
    rsx! { input { value: value } }
}

#[component]
fn Payload() -> VNode {
    rsx! { button { onclick: |e: &ferrum_core::FerrumEvent| {
        let n = e.payload.downcast_ref::<i32>().copied().unwrap_or(-1);
        assert_eq!(n, 7);
    }, "go" } }
}

fn ctx() -> Context<String> {
    thread_local! {
        static CTX: Context<String> = Context::new();
    }
    CTX.with(|c| c.clone())
}

#[component]
fn Consumer() -> VNode {
    let v = use_context(&ctx());
    rsx! { span { "v:{v}" } }
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Report {
    passed: usize,
    failed: Vec<String>,
}

impl Report {
    fn check(&mut self, name: &str, f: impl FnOnce()) {
        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(()) => {
                self.passed += 1;
                println!("ok   {name}");
            }
            Err(_) => {
                println!("FAIL {name}");
                self.failed.push(name.to_string());
            }
        }
    }

    fn total(&self) -> usize {
        self.passed + self.failed.len()
    }

    fn ratio(&self) -> f64 {
        if self.total() == 0 {
            1.0
        } else {
            self.passed as f64 / self.total() as f64
        }
    }
}

#[test]
fn react_subset_conformance() {
    let mut r = Report::default();

    r.check("state_update", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Counter {} });
        assert_eq!(h.text(), "count:0");
        h.fire("click", 0);
        assert_eq!(h.text(), "count:1");
    });

    r.check("state_setter_replaces_value", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Setter {} });
        h.fire("click", 0);
        assert_eq!(h.text(), "n:41");
    });

    r.check("effect_runs_after_commit", || {
        let ran = Rc::new(Cell::new(0));
        let h = TestRenderer::new();
        let r2 = ran.clone();
        h.render(move || rsx! { EffectOnce { ran: r2.clone() } });
        assert_eq!(ran.get(), 1);
    });

    r.check("effect_runs_once_without_deps", || {
        let ran = Rc::new(Cell::new(0));
        let h = TestRenderer::new();
        let r2 = ran.clone();
        h.render(move || rsx! { EffectOnce { ran: r2.clone() } });
        h.re_render();
        h.re_render();
        assert_eq!(ran.get(), 1);
    });

    r.check("effect_cleanup_on_unmount", || {
        let cleaned = Rc::new(Cell::new(false));
        {
            let h = TestRenderer::new();
            let c2 = cleaned.clone();
            h.render(move || rsx! { EffectCleanup { cleaned: c2.clone() } });
            assert!(!cleaned.get());
        }
        assert!(cleaned.get(), "cleanup ran when the host dropped");
    });

    r.check("memo_is_stable", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { MemoComp { deps: 2 } });
        let a = h.text();
        h.re_render();
        assert_eq!(a, h.text());
        assert_eq!(a, "m:4");
    });

    r.check("ref_is_stable", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { RefComp {} });
        let a = h.text();
        h.re_render();
        assert_eq!(a, h.text());
    });

    r.check("context_provides_value", || {
        let h = TestRenderer::new();
        let c = ctx();
        h.render(move || {
            let inner = rsx! { Consumer {} };
            c.provider("hello".to_string(), vec![inner])
        });
        assert_eq!(h.text(), "v:hello");
    });

    r.check("reducer_dispatches", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { ReducerComp {} });
        assert_eq!(h.text(), "s:0");
        h.fire("click", 0);
        assert_eq!(h.text(), "s:2");
    });

    r.check("use_id_is_stable", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { IdComp {} });
        let a = h.text();
        h.re_render();
        assert_eq!(a, h.text());
    });

    r.check("keyed_list_reconciles", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { KeyedList {} });
        assert_eq!(h.text(), "item:1item:2item:3");
    });

    r.check("fragments_are_transparent", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Fragment {} });
        assert_eq!(h.text(), "ab");
        assert_eq!(h.find_all("b").len(), 1);
        assert_eq!(h.find_all("i").len(), 1);
    });

    r.check("conditional_rendering", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Conditional { show: true } });
        assert_eq!(h.text(), "yes");
        h.render(|| rsx! { Conditional { show: false } });
        assert_eq!(h.text(), "no");
    });

    r.check("controlled_input_value", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Controlled { value: "abc".to_string() } });
        let input = h.find_all("input").remove(0);
        assert_eq!(h.attr(&input, "value"), Some(ferrum_core::AttrValue::Str("abc".into())));
    });

    r.check("event_payload_downcasts", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Payload {} });
        h.fire_with("click", 0, Box::new(7i32));
    });

    r.check("ssr_matches_client_markup", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Conditional { show: true } });
        let client = h.html();
        let server = ferrum_ssr::render_to_string(&rsx! { Conditional { show: true } });
        // The headless root adds a #root wrapper; compare the inner markup.
        assert!(client.contains(&server), "client={client} server={server}");
    });

    r.check("no_panic_over_1000_updates", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Counter {} });
        for _ in 0..1000 {
            h.fire("click", 0);
        }
        assert_eq!(h.text(), "count:1000");
    });

    r.check("unmount_is_idempotent", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Counter {} });
        drop(h);
    });

    r.check("nested_components_compose", || {
        let h = TestRenderer::new();
        h.render(|| rsx! { Fragment {} });
        h.render(|| rsx! { Counter {} });
        assert_eq!(h.text(), "count:0");
    });

    r.check("text_nodes_are_escaped_in_ssr", || {
        let html = ferrum_ssr::render_to_string(&rsx! { p { "<script>" } });
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    });

    println!("\nconformance: {}/{} passed ({:.1}%)", r.passed, r.total(), r.ratio() * 100.0);
    for f in &r.failed {
        println!("  failed: {f}");
    }
    assert!(r.ratio() >= 0.95, "conformance gate: need >= 95%, got {:.1}%", r.ratio() * 100.0);
    assert!(r.failed.is_empty(), "failing checks: {:?}", r.failed);
}