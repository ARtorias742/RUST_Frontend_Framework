//! Phase 0.5 spike #3: cooperative scheduler with priority lanes + time
//! slicing under a frame budget.
//!
//! Run: `cargo run -p ferrum-spikes --bin spike-scheduler`
//!
//! Models what React's scheduler does (and what `useTransition` needs):
//! work is split into chunks; the scheduler runs chunks between yields and
//! gives up the frame when the budget (e.g. 5 ms) is exceeded. Lower lane
//! number = higher priority. Measures slices, yields and max slice time, and
//! compares against a naive run with no yielding.

use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Lane {
    Sync = 0,       // must run now (discrete events)
    Default = 1,    // normal render
    Transition = 2, // interruptible (useTransition)
}

#[derive(Clone, Copy)]
struct Work {
    lane: Lane,
    chunks: usize,
    chunk_units: usize,
}

/// One run of the scheduler. Returns per-slice timings.
fn run(items: &[Work], budget: Duration) -> (Duration, usize, Duration) {
    let mut slices = 0usize;
    let mut max_slice = Duration::ZERO;
    let start = Instant::now();

    // Stable priority: lower lane first, FIFO within a lane.
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by_key(|&i| items[i].lane);

    let mut slice_start = Instant::now();
    for &i in &order {
        let w = items[i];
        for c in 0..w.chunks {
            // Simulate one chunk of work.
            let mut acc = 0u64;
            for u in 0..w.chunk_units {
                acc = acc.wrapping_add(black_box(u as u64));
            }
            black_box(acc);
            let _ = c;
            // Check the frame budget between chunks.
            if slice_start.elapsed() >= budget {
                slices += 1;
                let d = slice_start.elapsed();
                if d > max_slice {
                    max_slice = d;
                }
                slice_start = Instant::now();
            }
        }
    }
    // Final partial slice.
    let tail = slice_start.elapsed();
    if tail > Duration::ZERO {
        slices += 1;
        if tail > max_slice {
            max_slice = tail;
        }
    }
    let total = start.elapsed();
    (total, slices, max_slice)
}

fn main() {
    let budget = Duration::from_millis(5);

    // 2000 items across lanes; transitions are the bulk (interruptible),
    // plus some sync + default work, and a few very heavy items.
    let mut items = Vec::new();
    for i in 0..2000usize {
        let lane = match i % 10 {
            0 => Lane::Sync,
            1..=4 => Lane::Default,
            _ => Lane::Transition,
        };
        let chunks = 1 + (i % 7);
        let chunk_units = if i % 500 == 0 { 20_000 } else { 200 };
        items.push(Work {
            lane,
            chunks,
            chunk_units,
        });
    }

    let (total, slices, max_slice) = run(&items, budget);
    // Naive: run everything without yielding (budget = total time).
    let (naive_total, naive_slices, naive_max) = run(&items, Duration::from_secs(3600));

    println!("== spike-scheduler ==");
    println!(
        "work items:          {} (sync/default/transition lanes)",
        items.len()
    );
    println!("frame budget:        {budget:?}");
    println!("-- scheduled --");
    println!("total time:          {:.2} ms", total.as_secs_f64() * 1e3);
    println!("yields (slices):     {slices}");
    println!(
        "max slice time:      {:.2} ms",
        max_slice.as_secs_f64() * 1e3
    );
    println!("-- naive (no yield) --");
    println!(
        "total time:          {:.2} ms",
        naive_total.as_secs_f64() * 1e3
    );
    println!("yields (slices):     {naive_slices}");
    println!(
        "max slice time:      {:.2} ms  (one long block = jank)",
        naive_max.as_secs_f64() * 1e3
    );
    println!("verdict:             time slicing bounds the max slice to ~budget");
    assert!(
        naive_slices < slices,
        "scheduler must yield more than naive"
    );
    assert!(
        max_slice <= budget + Duration::from_millis(2),
        "max slice should stay near the budget"
    );
}
