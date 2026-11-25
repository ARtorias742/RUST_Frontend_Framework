//! Headless visual regression (the native half of Phase 9).
//!
//! Renders the component gallery through the *production* SSR path and
//! compares the HTML against a checked-in golden file. This catches markup
//! drift (a changed class, a dropped attribute, a reordered child) without a
//! browser; pixel-level regression lives in `e2e/tests/visual.spec.ts`.
//!
//! Regenerate baselines with `UPDATE_SNAPSHOTS=1 cargo test -p ferrum-devtools`.

use ferrum_components::{Badge, Button, Card, ThemeRoot};
use ferrum_core::{component, rsx};
use ferrum_styling::Theme;
use std::path::PathBuf;

#[component]
fn Gallery(dark: bool) -> VNode {
    let theme = if *dark { Theme::dark() } else { Theme::light() };
    rsx! {
        ThemeRoot { theme: theme,
            div {
                h1 { "Ferrum Gallery" }
                Card { title: "Card",
                    p { "Body text" }
                    Badge { label: "new" }
                }
                Button { variant: "primary", on_click: None, "Save" }
                Button { variant: "ghost", on_click: None, "Cancel" }
            }
        }
    }
}

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

fn assert_snapshot(name: &str, actual: &str) {
    let path = snapshot_path(name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "missing snapshot {}: {e} (run UPDATE_SNAPSHOTS=1)",
            path.display()
        )
    });
    assert_eq!(expected, actual, "snapshot {} drifted", name);
}

#[test]
fn gallery_light_matches_snapshot() {
    ferrum_styling::reset_registry();
    let html = ferrum_ssr::render_to_string(&rsx! { Gallery { dark: false } });
    assert_snapshot("gallery-light.html", &html);
}

#[test]
fn gallery_dark_matches_snapshot() {
    ferrum_styling::reset_registry();
    let html = ferrum_ssr::render_to_string(&rsx! { Gallery { dark: true } });
    assert_snapshot("gallery-dark.html", &html);
}

#[test]
fn snapshots_are_stable_across_runs() {
    ferrum_styling::reset_registry();
    let a = ferrum_ssr::render_to_string(&rsx! { Gallery { dark: false } });
    ferrum_styling::reset_registry();
    let b = ferrum_ssr::render_to_string(&rsx! { Gallery { dark: false } });
    assert_eq!(a, b, "scoped class hashes must be deterministic");
}
