//! Community app #3: a two-route blog with deep links (`/post/:id`).
//!
//! Shows the router + component library together: the list and the detail
//! page are separate route targets, and the detail page reads a param.

use ferrum_components::{Badge, Card, ThemeRoot};
use ferrum_core::{rsx, VNode};
use ferrum_router::{Params, RouteTarget, Router, Rules};
use ferrum_styling::Theme;
use ferrum_web::{AppHandle, BrowserHistory};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

struct Post {
    id: u32,
    title: &'static str,
    body: &'static str,
}

fn posts() -> Vec<Post> {
    vec![
        Post { id: 1, title: "Hello Ferrum", body: "Rust on the frontend." },
        Post { id: 2, title: "Hooks, explained", body: "State, effects and identity." },
        Post { id: 3, title: "Hydration", body: "From server HTML to live DOM." },
    ]
}

struct Home;
impl RouteTarget for Home {
    fn render(&self, _p: &Params, _o: VNode) -> VNode {
        let items: Vec<VNode> = posts()
            .iter()
            .map(|p| {
                let href = format!("/post/{}", p.id);
                let title = p.title;
                rsx! { li { a { href: href, "{title}" } } }
            })
            .collect();
        rsx! {
            ThemeRoot { theme: Theme::light(),
                div {
                    h1 { "Ferrum Blog" }
                    ul { {items} }
                }
            }
        }
    }
}

struct PostPage;
impl RouteTarget for PostPage {
    fn render(&self, p: &Params, _o: VNode) -> VNode {
        let id: u32 = p.get("id").and_then(|s| s.parse().ok()).unwrap_or(0);
        let post = posts().into_iter().find(|p| p.id == id);
        match post {
            Some(post) => rsx! {
                ThemeRoot { theme: Theme::light(),
                    Card { title: post.title,
                        Badge { label: "post" }
                        p { "{post.body}" }
                        a { href: "/", "← Back" }
                    }
                }
            },
            None => rsx! { p { "Post not found." } },
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let router = Router::new(
        Rules::new()
            .route("/", Home)
            .route("/post/:id", PostPage),
        BrowserHistory::new(),
    );
    let app = AppHandle::new(root);
    let r2 = router.clone();
    app.render(move || {
        let m = r2.current();
        m.target.render(&m.params, m.outlet.clone())
    });
    let a2 = app.clone();
    router.on_change(Rc::new(move || a2.re_render()));
}