//! # ferrum-reconciler
//!
//! Renderer-agnostic reconciliation planning.
//!
//! The DOM renderer ([`ferrum_web`]) uses [`plan_children`] to decide, for
//! each new child, whether it can be matched to an existing old child (by
//! key, or by position when unkeyed) or must be mounted fresh, and which old
//! children must be removed. The actual DOM mutation (reordering via
//! `insertBefore`, attribute diffing, listener churn) lives in the renderer.

use ferrum_core::Key;
use std::collections::HashMap;

mod renderer;
pub use renderer::{HydrationStats, ListenerHandle, NodeType, Renderer, TreeRenderer};

mod devtools;
pub use devtools::{observer, set_observer, Observer};

/// What to do with the new child at a given index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildOp {
    /// Reuse the old child at this index (update it in place).
    Reuse(usize),
    /// No match: mount a new child.
    Mount,
}

/// Result of planning a children update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildPlan {
    /// One op per new child, in order.
    pub ops: Vec<ChildOp>,
    /// Indices of old children that are no longer present and must be removed.
    pub remove: Vec<usize>,
}

/// Plan how to reconcile `old` children (by key) into `new` children.
///
/// - Keyed new children match keyed old children with the same key.
/// - Unkeyed new children match the next unused unkeyed old child, in order.
/// - Duplicate keys in `new` mount fresh (and warn in debug builds).
pub fn plan_children(old: &[Option<Key>], new: &[Option<Key>]) -> ChildPlan {
    let mut keyed: HashMap<Key, usize> = HashMap::new();
    for (i, k) in old.iter().enumerate() {
        if let Some(k) = k {
            keyed.insert(*k, i);
        }
    }

    let mut used = vec![false; old.len()];
    let mut next_unkeyed = 0usize;
    let mut ops = Vec::with_capacity(new.len());

    for k in new {
        match k {
            Some(key) => {
                if let Some(&j) = keyed.get(key) {
                    if !used[j] {
                        used[j] = true;
                        ops.push(ChildOp::Reuse(j));
                    } else {
                        // Duplicate key in `new`: mount fresh rather than reusing twice.
                        ops.push(ChildOp::Mount);
                    }
                } else {
                    ops.push(ChildOp::Mount);
                }
            }
            None => {
                while next_unkeyed < old.len()
                    && (used[next_unkeyed] || old[next_unkeyed].is_some())
                {
                    next_unkeyed += 1;
                }
                if next_unkeyed < old.len() {
                    used[next_unkeyed] = true;
                    ops.push(ChildOp::Reuse(next_unkeyed));
                    next_unkeyed += 1;
                } else {
                    ops.push(ChildOp::Mount);
                }
            }
        }
    }

    let remove = (0..old.len()).filter(|&i| !used[i]).collect();
    ChildPlan { ops, remove }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(v: u64) -> Option<Key> {
        Some(Key(v))
    }

    #[test]
    fn empty_old_all_mount() {
        let plan = plan_children(&[], &[k(1), k(2)]);
        assert_eq!(plan.ops, vec![ChildOp::Mount, ChildOp::Mount]);
        assert!(plan.remove.is_empty());
    }

    #[test]
    fn keyed_passthrough() {
        let plan = plan_children(&[k(1), k(2)], &[k(1), k(2)]);
        assert_eq!(plan.ops, vec![ChildOp::Reuse(0), ChildOp::Reuse(1)]);
    }

    #[test]
    fn keyed_reorder() {
        // [a, b, c] -> [c, a, b]
        let plan = plan_children(&[k(1), k(2), k(3)], &[k(3), k(1), k(2)]);
        assert_eq!(
            plan.ops,
            vec![ChildOp::Reuse(2), ChildOp::Reuse(0), ChildOp::Reuse(1)]
        );
        assert!(plan.remove.is_empty());
    }

    #[test]
    fn keyed_removal() {
        // [a, b, c] -> [a, c]
        let plan = plan_children(&[k(1), k(2), k(3)], &[k(1), k(3)]);
        assert_eq!(plan.ops, vec![ChildOp::Reuse(0), ChildOp::Reuse(2)]);
        assert_eq!(plan.remove, vec![1]);
    }

    #[test]
    fn keyed_insertion() {
        // [a, c] -> [a, b, c]
        let plan = plan_children(&[k(1), k(3)], &[k(1), k(2), k(3)]);
        assert_eq!(
            plan.ops,
            vec![ChildOp::Reuse(0), ChildOp::Mount, ChildOp::Reuse(1)]
        );
    }

    #[test]
    fn unkeyed_matches_by_position() {
        let plan = plan_children(&[None, None], &[None, None, None]);
        assert_eq!(
            plan.ops,
            vec![ChildOp::Reuse(0), ChildOp::Reuse(1), ChildOp::Mount]
        );
    }

    #[test]
    fn unkeyed_skips_keyed_old() {
        // old: [keyed a, unkeyed, unkeyed]; new: [unkeyed, unkeyed]
        let plan = plan_children(&[k(1), None, None], &[None, None]);
        assert_eq!(plan.ops, vec![ChildOp::Reuse(1), ChildOp::Reuse(2)]);
        assert_eq!(plan.remove, vec![0]);
    }

    #[test]
    fn mixed_keyed_and_unkeyed() {
        // old: [a, u1, b]; new: [u2, b, a]  (u2 unkeyed matches u1)
        let plan = plan_children(&[k(1), None, k(2)], &[None, k(2), k(1)]);
        assert_eq!(
            plan.ops,
            vec![ChildOp::Reuse(1), ChildOp::Reuse(2), ChildOp::Reuse(0)]
        );
        assert!(plan.remove.is_empty());
    }

    #[test]
    fn duplicate_new_key_mounts_fresh() {
        let plan = plan_children(&[k(1)], &[k(1), k(1)]);
        assert_eq!(plan.ops, vec![ChildOp::Reuse(0), ChildOp::Mount]);
    }
}
