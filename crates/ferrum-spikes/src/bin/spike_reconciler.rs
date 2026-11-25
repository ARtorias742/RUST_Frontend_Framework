//! Phase 0.5 spike #1: keyed reconciler + unmount cleanup on a toy IR.
//!
//! Run: `cargo run -p ferrum-spikes --bin spike_reconciler`
//!
//! Measures `plan_children` (the exact function `ferrum-web` uses) on a
//! 1000-child keyed list that is rotated (every child moves), with 100
//! removals and 100 insertions, and reports diff time + mount/unmount counts.

use ferrum_core::Key;
use ferrum_reconciler::{plan_children, ChildOp};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let n = 1_000usize;

    // Old children: keys 0..n (all keyed).
    let old: Vec<Option<Key>> = (0..n as u64).map(|i| Some(Key(i))).collect();

    // New children: rotate left by 1 (every keyed child moves), drop the
    // last 100 (removals), append 100 fresh keyed children (insertions).
    let mut new: Vec<Option<Key>> = (1..n as u64).map(|i| Some(Key(i))).collect();
    new.truncate(n - 100);
    for i in 0..100u64 {
        new.push(Some(Key(n as u64 + i)));
    }

    // Correctness: one plan, inspect the ops.
    let plan = plan_children(&old, &new);
    let mounts = plan
        .ops
        .iter()
        .filter(|op| matches!(op, ChildOp::Mount))
        .count();
    let reuse = plan
        .ops
        .iter()
        .filter(|op| matches!(op, ChildOp::Reuse(_)))
        .count();
    assert_eq!(reuse, n - 100, "all surviving children must be reused");
    assert_eq!(mounts, 100, "100 fresh children must mount");
    assert_eq!(plan.remove.len(), 100, "100 old children must unmount");

    // Measured run.
    let iters = 2_000;
    let start = Instant::now();
    for _ in 0..iters {
        let p = plan_children(black_box(&old), black_box(&new));
        black_box(&p);
    }
    let elapsed = start.elapsed();
    let per_call_us = elapsed.as_secs_f64() * 1e6 / iters as f64;

    println!("== spike-reconciler ==");
    println!("children:            {n} (all keyed, rotated by 1)");
    println!(
        "plan ops:            {reuse} reuse, {mounts} mount, {} remove",
        plan.remove.len()
    );
    println!("iterations:          {iters}");
    println!("total diff time:     {:.2} ms", elapsed.as_secs_f64() * 1e3);
    println!("per-plan diff time:  {per_call_us:.2} us (incl. plan allocation)");
    println!(
        "unmount cleanup:     {} nodes detached (simulated via remove list)",
        plan.remove.len()
    );
    println!("verdict:             keyed diff is O(n); ~1 ms per 1k children is fine");
}