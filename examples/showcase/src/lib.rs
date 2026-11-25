//! Community app #1: a component showcase (the "kitchen sink").
//!
//! Renders a broad slice of `ferrum-components` through the public API. It is
//! the app the Playwright visual suite points at (`/gallery`).

use ferrum_components::{
    Accordion, AccordionItem, Alert, Avatar, Badge, Breadcrumb, Button, Card, Checkbox, Divider,
    EmptyState, List, Progress, Select, Skeleton, Switch, TabItem, Table, Tabs, Textarea,
    ThemeRoot, Tooltip,
};
use ferrum_core::{component, rsx};
use ferrum_styling::Theme;
use ferrum_web::AppHandle;
use wasm_bindgen::prelude::*;

#[component]
fn Showcase() -> VNode {
    rsx! {
        ThemeRoot { theme: Theme::light(),
            div {
                Breadcrumb { items: vec!["Home".into(), "Components".into()] }
                h1 { "Ferrum Gallery" }
                Card { title: "Actions",
                    Button { variant: "primary", on_click: None, "Primary" }
                    Button { variant: "ghost", on_click: None, "Ghost" }
                    Badge { label: "v1.0" }
                    Avatar { name: "Ada Lovelace", src: "", size: "32px" }
                }
                Card { title: "Forms",
                    Textarea { value: "hello", placeholder: "Bio", rows: 3usize, on_input: None }
                    Select { options: vec!["a".to_string(), "b".to_string()], value: "a".to_string(), on_change: None }
                    Checkbox { checked: true, label: "Accept", on_change: None }
                    Switch { checked: false, label: "Dark mode", on_change: None }
                }
                Card { title: "Data",
                    Progress { value: 60, max: 100 }
                    Table {
                        columns: vec!["Name".into(), "Role".into()],
                        rows: vec![vec!["Ada".to_string(), "Engineer".to_string()]],
                    }
                    List { items: vec!["one".into(), "two".into()], ordered: false }
                }
                Card { title: "Navigation",
                    Tabs {
                        tabs: vec![
                            TabItem { label: "One".into(), content: rsx! { p { "first" } } },
                            TabItem { label: "Two".into(), content: rsx! { p { "second" } } },
                        ],
                        active: 0usize,
                        on_change: None,
                    }
                    Accordion {
                        items: vec![AccordionItem { title: "What is Ferrum?".into(), content: rsx! { p { "A Rust frontend framework." } } }],
                        open: 0usize,
                    }
                    Tooltip { text: "Hover me", span { "?" } }
                }
                Card { title: "Feedback",
                    Alert { level: "info", title: "Heads up", p { "This is an alert." } }
                    Skeleton { lines: 2usize, width: "80%" }
                }
                Divider { label: "end" }
                EmptyState { title: "Nothing here", description: "Try another route.", }
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let app = AppHandle::new(root);
    app.render(|| rsx! { Showcase {} });
    app.forget();
}
