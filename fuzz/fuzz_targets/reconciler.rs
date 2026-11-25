//! Fuzz keyed reconciliation: arbitrary key sequences must not panic and must
//! produce a plan covering every new child.
#![no_main]

use ferrum_core::Key;
use ferrum_reconciler::{plan_children, ChildOp};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let keys: Vec<Option<Key>> = data
        .iter()
        .map(|b| if *b % 3 == 0 { None } else { Some(Key(*b as u64)) })
        .collect();
    let plan = plan_children(&keys, &keys);
    assert_eq!(plan.ops.len(), keys.len());
    assert!(plan.ops.iter().all(|op| matches!(op, ChildOp::Reuse(_))));
    assert!(plan.remove.is_empty());

    // Reversed input must still cover every child exactly once.
    let mut reversed = keys.clone();
    reversed.reverse();
    let plan = plan_children(&keys, &reversed);
    assert_eq!(plan.ops.len(), reversed.len());
    assert_eq!(plan.ops.iter().filter(|op| matches!(op, ChildOp::Mount)).count(), 0);
});