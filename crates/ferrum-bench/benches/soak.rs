//! Soak benchmark: 10,000 state updates with a counting allocator.
//!
//! Run with `cargo bench -p ferrum-bench --bench soak`.
//! Gate (roadmap §4): no heap growth > 5% over the soak; the bench fails if
//! the live allocation grows beyond that after warm-up.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use ferrum_core::{component, rsx, use_state};
use ferrum_testing::TestRenderer;

struct Counting;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATED.fetch_add(new_size, Ordering::Relaxed);
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[component]
fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! { button { onclick: move |_| set.update(|c| c + 1), "count:{count}" } }
}

fn live() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

fn main() {
    let h = TestRenderer::new();
    h.render(|| rsx! { Counter {} });

    // Warm up: let the allocator settle and all one-shot allocations happen.
    for _ in 0..1_000 {
        h.fire("click", 0);
    }
    let baseline = live();

    let iterations = 10_000;
    for _ in 0..iterations {
        h.fire("click", 0);
    }
    let after = live();
    let growth = after.saturating_sub(baseline);
    let pct = growth as f64 / baseline.max(1) as f64 * 100.0;

    println!(
        "soak: {iterations} updates, live heap {baseline} B -> {after} B ({growth} B, {pct:.2}%)"
    );
    assert!(
        pct <= 5.0,
        "heap growth {pct:.2}% exceeds the 5% soak budget ({growth} B)"
    );
    assert_eq!(h.text(), format!("count:{}", 1_000 + iterations));
}
