//! Reconciliation benchmark: keyed list of 1,000 children, reordered.
//!
//! Run with `cargo bench -p ferrum-bench --bench reconcile`.
//! Budget (roadmap §4): a 1k-child keyed reorder plans in well under 16.6 ms
//! (one 60 fps frame).

use ferrum_core::{component, rsx, VNode};
use ferrum_testing::TestRenderer;
use std::time::Instant;

#[component]
fn Item(id: u64) -> VNode {
    rsx! { li { key: *id, "item:{id}" } }
}

#[component]
fn List(order: Vec<u64>) -> VNode {
    let items: Vec<VNode> = order.iter().map(|id| rsx! { Item { id: *id } }).collect();
    rsx! { ul { {items} } }
}

fn main() {
    let n = 1_000usize;
    let base: Vec<u64> = (0..n as u64).collect();
    let mut reversed = base.clone();
    reversed.reverse();

    // Warm up (also pays first-render costs).
    let h = TestRenderer::new();
    let warm = base.clone();
    h.render(move || rsx! { List { order: warm.clone() } });

    let start = Instant::now();
    let iterations = 20;
    for _ in 0..iterations {
        let r = reversed.clone();
        h.render(move || rsx! { List { order: r.clone() } });
        let b = base.clone();
        h.render(move || rsx! { List { order: b.clone() } });
    }
    let elapsed = start.elapsed();

    let per_update = elapsed.as_secs_f64() * 1000.0 / (iterations * 2) as f64;
    println!(
        "reconcile: {n} keyed children, {} updates in {:.2} ms ({:.3} ms/update, {:.1} fps headroom)",
        iterations * 2,
        elapsed.as_secs_f64() * 1000.0,
        per_update,
        1000.0 / per_update
    );
    assert!(per_update < 16.6, "reconcile budget exceeded: {per_update:.3} ms");
}