use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Stable identity for a child inside a list.
///
/// Keys let the reconciler match old children to new children across updates
/// (moves, insertions, removals) instead of re-mounting by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub u64);

impl Key {
    /// Hash any hashable value into a key (used for `&str` / `String` keys).
    pub fn hash_of<T: Hash>(value: &T) -> Self {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Key(hasher.finish())
    }
}

impl From<u64> for Key {
    fn from(v: u64) -> Self {
        Key(v)
    }
}
impl From<u32> for Key {
    fn from(v: u32) -> Self {
        Key(v as u64)
    }
}
impl From<i32> for Key {
    fn from(v: i32) -> Self {
        Key(v as u64)
    }
}
impl From<usize> for Key {
    fn from(v: usize) -> Self {
        Key(v as u64)
    }
}
impl From<&str> for Key {
    fn from(v: &str) -> Self {
        Key::hash_of(&v)
    }
}
impl From<String> for Key {
    fn from(v: String) -> Self {
        Key::hash_of(&v)
    }
}
impl From<&String> for Key {
    fn from(v: &String) -> Self {
        Key::hash_of(v)
    }
}
