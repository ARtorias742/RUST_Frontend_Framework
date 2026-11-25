//! Shared component used by both the web and native examples (Phase 8 gate:
//! one component model, two renderers).

use ferrum_core::{component, rsx, use_state};

#[component]
pub fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    let set = set.clone();
    rsx! {
        div { class: "counter",
            button { class: "inc", onclick: move |_| set.update(|c| c + 1), "+" }
            span { class: "value", "Count: {count}" }
        }
    }
}