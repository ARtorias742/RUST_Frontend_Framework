//! Cold-start benchmark: time from an empty host to a fully mounted 1,000-node
//! tree (the "first render" budget).
//!
//! Run with `cargo bench -p ferrum-bench --bench startup`.
//! Budget (roadmap §4): TTI < 1.5 s on mid-tier mobile; the Rust-side mount
//! must be a small fraction of that.

use ferrum_core::{component, rsx, VNode};
use ferrum_testing::TestRenderer;
use std::time::Instant;

#[component]
fn Row(i: u64) -> VNode {
    rsx! { div { class: "row", span { "row:{i}" } } }
}

fn main() {
    let n = 1_000u64;
    let start = Instant::now();
    let h = TestRenderer::new();
    let rows: Vec<VNode> = (0..n).map(|i| rsx! { Row { i: i } }).collect();
    h.render(move || rsx! { div { {rows.clone()} } });
    let elapsed = start.elapsed();

    println!(
        "startup: mounted {n} rows in {:.3} ms ({:.1} us/row)",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_secs_f64() * 1_000_000.0 / n as f64
    );
    assert!(elapsed.as_secs_f64() < 1.5, "startup budget exceeded");
}