//! Phase 3 demo (native): hooks running on the headless TestRenderer.
//!
//! Run: `cargo run -p example-hooks`
//!
//! Shows useReducer + useEffect + useContext + useMemo + useId working
//! together, with events fired and effects flushed exactly like the browser
//! renderer does.

use ferrum_core::{
    component, rsx, use_context, use_effect, use_id, use_memo, use_reducer, Context,
};
use ferrum_testing::TestRenderer;
use std::cell::Cell;
use std::rc::Rc;

// A theme context shared by the whole tree.
thread_local! {
    static THEME: Context<String> = Context::new();
}

fn theme() -> Context<String> {
    THEME.with(|t| t.clone())
}

#[component]
fn ThemedButton(label: String, on_click: Rc<dyn Fn(&ferrum_core::FerrumEvent)>) -> VNode {
    let th = theme();
    let theme_name = use_context(&th);
    let id = use_id();
    let on_click = on_click.clone();
    rsx! {
        button { class: "btn {theme_name}", data-id: id, onclick: move |e| on_click(e), "{label}" }
    }
}

#[component]
fn Counter() -> VNode {
    let (count, dispatch) = use_reducer(|s: &i32, a: i32| s + a, 0);
    // useMemo: double the count (recomputed only when count changes).
    let doubled = use_memo(&[count], || count * 2);
    // useEffect: log every count change (runs after commit).
    let last = Rc::new(Cell::new(count));
    let last2 = last.clone();
    use_effect(
        move || {
            let prev = last2.get();
            last2.set(count);
            if prev != count {
                println!("  [effect] count changed: {prev} -> {count}");
            }
            None
        },
        Some(&[count]),
    );
    rsx! {
        div { class: "counter",
            ThemedButton { label: "+", on_click: Rc::new(move |_: &ferrum_core::FerrumEvent| dispatch.clone().dispatch(1)) as Rc<dyn Fn(&ferrum_core::FerrumEvent)> }
            span { class: "value", "Count: {count} (doubled: {doubled})" }
        }
    }
}

fn main() {
    let r = TestRenderer::new();
    r.render(move || theme().provider("dark".to_string(), vec![rsx! { Counter {} }]));

    println!("initial:      {}", r.text());
    for i in 0..3 {
        r.fire("click", 0);
        println!("after click {}: {}", i + 1, r.text());
    }

    // useId is stable and unique per hook slot.
    let buttons = r.find_all("button");
    let id1 = r.attr(&buttons[0], "data-id");
    r.re_render();
    let buttons2 = r.find_all("button");
    let id2 = r.attr(&buttons2[0], "data-id");
    println!("use_id stable across re-render: {}", id1 == id2);

    // Class from context.
    let class = r.attr(&buttons2[0], "class");
    println!("context class: {:?}", class);
}
