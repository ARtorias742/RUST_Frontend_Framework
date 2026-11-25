//! Phase 1 example: `rsx!` + component IR + debug renderer (no DOM).
//!
//! Run with: `cargo run -p example-counter`

use ferrum_core::{component, render_debug, rsx};

#[component]
fn Counter(count: i32, step: i32) -> VNode {
    rsx! {
        div { class: "counter",
            button { class: "inc", onclick: move |_| {}, "+{step}" }
            span { class: "value", "Count: {count}" }
        }
    }
}

fn main() {
    let tree = rsx! {
        div { class: "app",
            h1 { "Ferrum Counter (Phase 1: IR only)" }
            Counter { count: 3, step: 1 }
            Counter { count: 10, step: 5 }
        }
    };
    println!("{}", render_debug(&tree));
}
