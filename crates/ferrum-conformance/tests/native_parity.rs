//! Phase 4 gate: promoted components render the same tree on headless and
//! native backends.

use ferrum_components::{
    Calendar, Command, DatePicker, HoverCard, NavItem, NavigationMenu, Popover, Sheet, Stat,
    Timeline, TimelineItem, Tree, TreeNode,
};
use ferrum_core::{rsx, VNode};
use ferrum_native::NativeApp;
use ferrum_testing::TestRenderer;

fn assert_parity(node: VNode) {
    let headless = TestRenderer::new();
    let n1 = node.clone();
    headless.render(move || n1.clone());
    let native = NativeApp::new();
    let n2 = node.clone();
    native.render(move || n2.clone());

    assert_eq!(headless.text(), native.text());
    assert!(!headless.html().is_empty());
    assert!(!native.html().is_empty());
    assert_eq!(
        headless.html().contains("role="),
        native.html().contains("role=")
    );
}

#[test]
fn stat_parity() {
    assert_parity(rsx! { Stat { label: "Revenue", value: "$12k", delta: "+12%" } });
}

#[test]
fn command_parity() {
    assert_parity(rsx! {
        Command { placeholder: "Search", value: "a", on_input: None, items: vec!["Ada".to_string(), "Bob".to_string()] }
    });
}

#[test]
fn sheet_parity() {
    assert_parity(rsx! { Sheet { open: true, title: "Settings", on_close: None, "Body" } });
}

#[test]
fn popover_parity() {
    assert_parity(rsx! { Popover { open: true, "Panel" } });
}

#[test]
fn hover_card_parity() {
    assert_parity(rsx! { HoverCard { text: "More", span { "?" } } });
}

#[test]
fn navigation_menu_parity() {
    let items = vec![
        NavItem {
            label: "Home".into(),
            href: "/".into(),
        },
        NavItem {
            label: "Docs".into(),
            href: "/docs".into(),
        },
    ];
    assert_parity(rsx! { NavigationMenu { items: items } });
}

#[test]
fn tree_parity() {
    let nodes = vec![TreeNode {
        label: "src".into(),
        children: vec![TreeNode {
            label: "main.rs".into(),
            children: vec![],
        }],
    }];
    assert_parity(rsx! { Tree { nodes: nodes } });
}

#[test]
fn timeline_parity() {
    let items = vec![TimelineItem {
        title: "Now".into(),
        detail: "Ship".into(),
    }];
    assert_parity(rsx! { Timeline { items: items } });
}

#[test]
fn calendar_parity() {
    assert_parity(rsx! { Calendar { month: "November", days: vec![1, 2, 3] } });
}

#[test]
fn date_picker_parity() {
    assert_parity(rsx! { DatePicker { value: "2025-01-01", on_change: None } });
}
