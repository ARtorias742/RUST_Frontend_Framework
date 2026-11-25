//! # ferrum-ui-core
//!
//! Headless behavior shared by Ferrum UI components. This crate is
//! renderer-agnostic: it contains no DOM, CSS or platform code.
//!
//! - [`FocusScope`] implements the focus-trap/restore bookkeeping for
//!   dialogs, menus and overlays.
//! - [`RovingTabIndex`] implements the “one tab stop” pattern for tabs,
//!   menus and toolbars.
//! - [`PortalHost`] models named overlay roots so web/native hosts can
//!   attach the same component tree to different layers.
//!
//! Styled source lives in the user's app (`ferrum add`); this crate holds
//! the behavior that is genuinely shared and hard to copy.

/// A named focus scope (dialog, menu, popover).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusScope {
    id: u64,
    name: String,
}

impl FocusScope {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        FocusScope {
            id,
            name: name.into(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The focusable ids in tab order, de-duplicated.
    pub fn focusable(&self, ids: &[u64]) -> Vec<u64> {
        let mut out = Vec::new();
        for id in ids {
            if !out.contains(id) {
                out.push(*id);
            }
        }
        out
    }

    pub fn first(&self, ids: &[u64]) -> Option<u64> {
        self.focusable(ids).first().copied()
    }

    pub fn last(&self, ids: &[u64]) -> Option<u64> {
        self.focusable(ids).last().copied()
    }

    /// Next focusable id, wrapping at the end.
    pub fn next(&self, current: Option<u64>, ids: &[u64]) -> Option<u64> {
        let ids = self.focusable(ids);
        if ids.is_empty() {
            return None;
        }
        let pos = current
            .and_then(|id| ids.iter().position(|candidate| *candidate == id))
            .map(|p| p + 1)
            .unwrap_or(0);
        Some(ids[pos % ids.len()])
    }

    /// Previous focusable id, wrapping at the start.
    pub fn previous(&self, current: Option<u64>, ids: &[u64]) -> Option<u64> {
        let ids = self.focusable(ids);
        if ids.is_empty() {
            return None;
        }
        let pos = match current.and_then(|id| ids.iter().position(|candidate| *candidate == id)) {
            Some(0) | None => ids.len() - 1,
            Some(p) => p - 1,
        };
        Some(ids[pos])
    }

    /// `true` if `id` belongs to this scope.
    pub fn contains(&self, id: u64) -> bool {
        self.id == id
    }
}

/// Roving tabindex state: only the active item is tabbable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RovingTabIndex {
    active: usize,
}

impl RovingTabIndex {
    pub fn new(active: usize) -> Self {
        RovingTabIndex { active }
    }

    pub fn active(&self) -> usize {
        self.active
    }

    pub fn set(&mut self, active: usize) {
        self.active = active;
    }

    /// Move by `delta`, clamping to `len - 1` (no wrap).
    pub fn move_by(&mut self, len: usize, delta: isize) {
        if len == 0 {
            self.active = 0;
            return;
        }
        let next = self.active as isize + delta;
        self.active = next.clamp(0, len as isize - 1) as usize;
    }

    pub fn home(&mut self) {
        self.active = 0;
    }

    pub fn end(&mut self, len: usize) {
        self.active = len.saturating_sub(1);
    }

    /// The `tabindex` value for item `index`: `0` for active, `-1` otherwise.
    pub fn tab_index(&self, index: usize) -> isize {
        if index == self.active {
            0
        } else {
            -1
        }
    }
}

/// A named overlay root. Hosts map names to real layers (DOM body, native
/// overlay layer, test root).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PortalHost {
    entries: Vec<PortalEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PortalEntry {
    name: String,
    open: bool,
}

impl PortalHost {
    pub fn new() -> Self {
        PortalHost::default()
    }

    /// Register a portal root (idempotent).
    pub fn mount(&mut self, name: impl Into<String>) {
        let name = name.into();
        if !self.entries.iter().any(|entry| entry.name == name) {
            self.entries.push(PortalEntry { name, open: true });
        }
    }

    pub fn unmount(&mut self, name: &str) {
        self.entries.retain(|entry| entry.name != name);
    }

    pub fn set_open(&mut self, name: &str, open: bool) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.name == name) {
            entry.open = open;
        }
    }

    pub fn is_open(&self, name: &str) -> bool {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.open)
            .unwrap_or(false)
    }

    pub fn names(&self) -> Vec<&str> {
        self.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_scope_wraps_and_dedupes() {
        let scope = FocusScope::new(1, "dialog");
        assert_eq!(scope.first(&[3, 3, 5, 7]), Some(3));
        assert_eq!(scope.next(Some(7), &[3, 5, 7]), Some(3));
        assert_eq!(scope.previous(Some(3), &[3, 5, 7]), Some(7));
        assert_eq!(scope.next(None, &[]), None);
    }

    #[test]
    fn roving_tabindex_moves_and_clamps() {
        let mut roving = RovingTabIndex::new(0);
        roving.move_by(3, 1);
        assert_eq!(roving.active(), 1);
        assert_eq!(roving.tab_index(1), 0);
        assert_eq!(roving.tab_index(0), -1);
        roving.end(3);
        assert_eq!(roving.active(), 2);
        roving.move_by(3, 1);
        assert_eq!(roving.active(), 2);
    }

    #[test]
    fn portal_host_tracks_open_roots() {
        let mut host = PortalHost::new();
        host.mount("overlay");
        host.mount("overlay");
        assert!(host.is_open("overlay"));
        host.set_open("overlay", false);
        assert!(!host.is_open("overlay"));
        host.unmount("overlay");
        assert!(!host.is_open("overlay"));
        assert_eq!(host.names(), Vec::<&str>::new());
    }
}
