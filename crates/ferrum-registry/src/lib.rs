//! # ferrum-registry
//!
//! The component registry and the semver/deprecation rules that go with it.
//!
//! Ferrum 1.0 freezes the public API. This crate encodes the policy in code
//! so `ferrum release` (and CI) can check a release mechanically:
//!
//! - [`Version`] is a strict `MAJOR.MINOR.PATCH` parser.
//! - [`Deprecation`] is `Active`, `Deprecated { since, message }`, or
//!   `Removed { since }`.
//! - [`Registry::diff`] classifies changes between two registries as
//!   **breaking** (removed/renamed), **minor** (added/deprecated), or
//!   **patch** (metadata only).
//!
//! Policy (see `docs/governance.md`):
//!
//! | Change | Allowed in |
//! |---|---|
//! | Add a component | minor |
//! | Deprecate a component | minor (with migration note) |
//! | Remove a component | major (or after 2 minors deprecated) |
//! | Change a component's category | minor |
//! | Fix docs/metadata | patch |
//!
//! ```rust
//! use ferrum_registry::{Registry, Version};
//!
//! let mut a = Registry::new();
//! a.register("Button", Version::new(1, 0, 0), "action");
//! let mut b = a.clone();
//! b.deprecate("Button", Version::new(1, 1, 0), "use Action");
//! assert_eq!(a.diff(&b), vec![ferrum_registry::Change::Deprecated("Button".into())]);
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// A strict semantic version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Version {
            major,
            minor,
            patch,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.major == 0 && self.minor == 0 && self.patch == 0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Parse `MAJOR.MINOR.PATCH` (no pre-release/build metadata).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVersionError(pub String);

impl fmt::Display for ParseVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid semantic version: {}", self.0)
    }
}

impl std::error::Error for ParseVersionError {}

impl FromStr for Version {
    type Err = ParseVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
        {
            return Err(ParseVersionError(s.to_string()));
        }
        let parse = |p: &str| {
            p.parse::<u64>()
                .map_err(|_| ParseVersionError(s.to_string()))
        };
        Ok(Version {
            major: parse(parts[0])?,
            minor: parse(parts[1])?,
            patch: parse(parts[2])?,
        })
    }
}

// ---------------------------------------------------------------------------
// Deprecation
// ---------------------------------------------------------------------------

/// The lifecycle state of a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deprecation {
    Active,
    Deprecated { since: Version, message: String },
    Removed { since: Version },
}

impl Deprecation {
    pub fn is_active(&self) -> bool {
        matches!(self, Deprecation::Active)
    }

    /// The version at which the component left the active set.
    pub fn since(&self) -> Option<Version> {
        match self {
            Deprecation::Active => None,
            Deprecation::Deprecated { since, .. } | Deprecation::Removed { since } => Some(*since),
        }
    }
}

// ---------------------------------------------------------------------------
// Entries + registry
// ---------------------------------------------------------------------------

/// One registered component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub version: Version,
    pub category: String,
    pub deprecation: Deprecation,
}

/// A snapshot of the registry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Registry {
    entries: BTreeMap<String, Entry>,
}

impl Registry {
    pub fn new() -> Self {
        Registry::default()
    }

    /// Add or update a component.
    pub fn register(&mut self, name: &str, version: Version, category: &str) -> &mut Self {
        self.entries.insert(
            name.to_string(),
            Entry {
                name: name.to_string(),
                version,
                category: category.to_string(),
                deprecation: Deprecation::Active,
            },
        );
        self
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.entries.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Mark a component deprecated (no-op if unknown).
    pub fn deprecate(&mut self, name: &str, since: Version, message: &str) -> &mut Self {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.deprecation = Deprecation::Deprecated {
                since,
                message: message.to_string(),
            };
        }
        self
    }

    /// Mark a component removed (no-op if unknown).
    pub fn remove(&mut self, name: &str, since: Version) -> &mut Self {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.deprecation = Deprecation::Removed { since };
        }
        self
    }

    /// Drop a component entirely (a major-version breaking change).
    pub fn delete(&mut self, name: &str) -> &mut Self {
        self.entries.remove(name);
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }

    pub fn active(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values().filter(|e| e.deprecation.is_active())
    }

    /// Classify the changes needed to go from `self` to `next`.
    pub fn diff(&self, next: &Registry) -> Vec<Change> {
        let mut changes = Vec::new();
        for (name, old) in &self.entries {
            match next.entries.get(name) {
                None => changes.push(Change::Removed(name.clone())),
                Some(new) => {
                    if old.deprecation.is_active() && !new.deprecation.is_active() {
                        changes.push(Change::Deprecated(name.clone()));
                    }
                    if old.category != new.category {
                        changes.push(Change::Recategorized(name.clone()));
                    }
                    if old.version != new.version {
                        changes.push(Change::VersionChanged {
                            name: name.clone(),
                            from: old.version,
                            to: new.version,
                        });
                    }
                }
            }
        }
        for name in next.entries.keys() {
            if !self.entries.contains_key(name) {
                changes.push(Change::Added(name.clone()));
            }
        }
        changes.sort_by_key(|c| c.name().to_string());
        changes
    }

    /// `true` if `diff` contains a breaking change.
    pub fn is_breaking(&self, next: &Registry) -> bool {
        self.diff(next).iter().any(Change::is_breaking)
    }
}

/// A classified registry change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Added(String),
    Removed(String),
    Deprecated(String),
    Recategorized(String),
    VersionChanged {
        name: String,
        from: Version,
        to: Version,
    },
}

impl Change {
    pub fn name(&self) -> &str {
        match self {
            Change::Added(n)
            | Change::Removed(n)
            | Change::Deprecated(n)
            | Change::Recategorized(n) => n,
            Change::VersionChanged { name, .. } => name,
        }
    }

    /// Removing a component is the only breaking change in the 1.x policy.
    pub fn is_breaking(&self) -> bool {
        matches!(self, Change::Removed(_))
    }

    pub fn is_minor(&self) -> bool {
        matches!(
            self,
            Change::Added(_) | Change::Deprecated(_) | Change::Recategorized(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_orders_versions() {
        assert_eq!("1.2.3".parse::<Version>().unwrap(), Version::new(1, 2, 3));
        assert!("1.2".parse::<Version>().is_err());
        assert!("1.2.3.4".parse::<Version>().is_err());
        assert!("a.b.c".parse::<Version>().is_err());
        assert!(Version::new(1, 2, 3) < Version::new(1, 10, 0));
    }

    #[test]
    fn diff_classifies_changes() {
        let mut a = Registry::new();
        a.register("Button", Version::new(1, 0, 0), "action");
        a.register("Card", Version::new(1, 0, 0), "layout");

        let mut b = a.clone();
        b.register("Badge", Version::new(1, 0, 0), "data-display");
        b.deprecate("Card", Version::new(1, 1, 0), "use Panel");
        b.delete("Button");

        let diff = a.diff(&b);
        assert!(diff.contains(&Change::Added("Badge".into())));
        assert!(diff.contains(&Change::Deprecated("Card".into())));
        assert!(diff.contains(&Change::Removed("Button".into())));
        assert!(a.is_breaking(&b));
    }

    #[test]
    fn deprecation_lifecycle() {
        let mut r = Registry::new();
        r.register("X", Version::new(1, 0, 0), "misc");
        r.deprecate("X", Version::new(1, 1, 0), "gone soon");
        assert_eq!(
            r.get("X").unwrap().deprecation.since(),
            Some(Version::new(1, 1, 0))
        );
        r.remove("X", Version::new(2, 0, 0));
        assert!(matches!(
            r.get("X").unwrap().deprecation,
            Deprecation::Removed { .. }
        ));
    }
}

// ---------------------------------------------------------------------------
// Template registry (ferrum add)
// ---------------------------------------------------------------------------

/// One installable UI component template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateEntry {
    pub name: String,
    pub version: Version,
    pub category: String,
    /// Files written relative to the target directory.
    pub files: Vec<String>,
    /// Crates the generated source depends on.
    pub deps: Vec<String>,
    pub deprecation: Deprecation,
}

/// A versioned registry of copy-paste component templates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplateRegistry {
    entries: BTreeMap<String, TemplateEntry>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        TemplateRegistry::default()
    }

    pub fn register(
        &mut self,
        name: &str,
        version: Version,
        category: &str,
        files: &[&str],
        deps: &[&str],
    ) -> &mut Self {
        self.entries.insert(
            name.to_string(),
            TemplateEntry {
                name: name.to_string(),
                version,
                category: category.to_string(),
                files: files.iter().map(|f| f.to_string()).collect(),
                deps: deps.iter().map(|d| d.to_string()).collect(),
                deprecation: Deprecation::Active,
            },
        );
        self
    }

    pub fn get(&self, name: &str) -> Option<&TemplateEntry> {
        self.entries.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &TemplateEntry> {
        self.entries.values()
    }

    pub fn deprecate(&mut self, name: &str, since: Version, message: &str) -> &mut Self {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.deprecation = Deprecation::Deprecated {
                since,
                message: message.to_string(),
            };
        }
        self
    }

    pub fn remove_template(&mut self, name: &str) -> &mut Self {
        self.entries.remove(name);
        self
    }
}
