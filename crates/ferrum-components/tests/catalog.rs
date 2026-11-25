//! Phase 10 gate: the component catalog is complete, unique and renderable.

use ferrum_components::CATALOG;

#[test]
fn catalog_has_forty_components() {
    assert_eq!(CATALOG.len(), 40, "the 1.0 library ships 40 components");
}

#[test]
fn catalog_names_are_unique_and_non_empty() {
    let mut names: Vec<&str> = CATALOG.iter().map(|c| c.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), CATALOG.len(), "duplicate component name");
    assert!(CATALOG.iter().all(|c| !c.category.is_empty()));
    assert!(CATALOG.iter().all(|c| !c.since.is_empty()));
}

#[test]
fn catalog_covers_every_category() {
    let categories: std::collections::BTreeSet<&str> = CATALOG.iter().map(|c| c.category).collect();
    for expected in [
        "layout",
        "action",
        "form",
        "data-display",
        "feedback",
        "navigation",
        "overlay",
    ] {
        assert!(categories.contains(expected), "missing category {expected}");
    }
}
