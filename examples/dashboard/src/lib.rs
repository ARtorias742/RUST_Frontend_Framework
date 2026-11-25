//! Community app #2: a small analytics dashboard with a data table and
//! progress meters. Demonstrates the component library without a router.

use ferrum_components::{Badge, Button, Card, Progress, Table, ThemeRoot};
use ferrum_core::{component, rsx, use_state};
use ferrum_styling::Theme;
use ferrum_web::AppHandle;
use wasm_bindgen::prelude::*;

#[component]
fn Dashboard() -> VNode {
    let (range, _set) = use_state(|| "7d".to_string());
    let rows = vec![
        vec!["Ada".to_string(), "Engineer".to_string(), "42".to_string()],
        vec!["Bob".to_string(), "Designer".to_string(), "38".to_string()],
        vec!["Clara".to_string(), "PM".to_string(), "51".to_string()],
    ];
    rsx! {
        ThemeRoot { theme: Theme::light(),
            div {
                h1 { "Dashboard" }
                p { "Range: {range}" }
                Button { variant: "ghost", on_click: None, "7 days" }
                Button { variant: "primary", on_click: None, "30 days" }
                Card { title: "Traffic",
                    Progress { value: 72, max: 100 }
                    Badge { label: "+12%" }
                }
                Card { title: "Team",
                    Table { columns: vec!["Name".into(), "Role".into(), "Commits".into()], rows: rows }
                }
                Card { title: "Uptime", Progress { value: 99, max: 100 } }
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    AppHandle::new(root).render(|| rsx! { Dashboard {} });
}
