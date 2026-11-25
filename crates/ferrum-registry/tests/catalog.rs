//! Phase 10 gate: the registry accepts the shipped catalog and enforces the
//! semver/deprecation policy.

use ferrum_components::CATALOG;
use ferrum_registry::{Change, Registry, Version};

fn shipped() -> Registry {
    let mut r = Registry::new();
    for c in CATALOG {
        let version = if c.since == "0.1" {
            Version::new(0, 1, 0)
        } else {
            Version::new(1, 0, 0)
        };
        r.register(c.name, version, c.category);
    }
    r
}

#[test]
fn registry_loads_the_full_catalog() {
    let r = shipped();
    assert_eq!(r.len(), 30);
    assert!(r.contains("Button"));
    assert_eq!(r.active().count(), 30);
}

#[test]
fn adding_a_component_is_a_minor_change() {
    let a = shipped();
    let mut b = a.clone();
    b.register("Command", Version::new(1, 1, 0), "action");
    let diff = a.diff(&b);
    assert_eq!(diff, vec![Change::Added("Command".into())]);
    assert!(!a.is_breaking(&b));
    assert!(diff.iter().all(Change::is_minor));
}

#[test]
fn removing_a_component_is_breaking() {
    let a = shipped();
    let mut b = a.clone();
    b.delete("Divider");
    assert!(a.is_breaking(&b));
    assert_eq!(a.diff(&b), vec![Change::Removed("Divider".into())]);
}

#[test]
fn deprecating_requires_a_migration_note() {
    let a = shipped();
    let mut b = a.clone();
    b.deprecate("Badge", Version::new(1, 2, 0), "use Tag");
    let entry = b.get("Badge").unwrap();
    match &entry.deprecation {
        ferrum_registry::Deprecation::Deprecated { since, message } => {
            assert_eq!(*since, Version::new(1, 2, 0));
            assert_eq!(message, "use Tag");
        }
        other => panic!("expected deprecated, got {other:?}"),
    }
    assert!(!a.is_breaking(&b), "deprecation is not breaking");
}

#[test]
fn registry_is_deterministic() {
    let a = shipped();
    let b = shipped();
    assert_eq!(a, b);
}
