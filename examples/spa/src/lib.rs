//! Phase 4 gate (wasm): 4-page SPA with deep links, nested layout, redirect,
//! and async data via ferrum-data.
//!
//! Pages: `/` (redirect -> /home), `/home`, `/users` (async list),
//! `/users/:id` (async detail), `/about`, and a nested `/panel` layout.
//! Serve with `trunk serve`; the page needs `<div id="app"></div>`.

use ferrum_core::{rsx, VNode};
use ferrum_data::{use_query, Client, QuerySnapshot};
use ferrum_router::{Params, RouteTarget, Router, Rules};
use ferrum_web::{AppHandle, BrowserHistory};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[derive(Clone)]
struct User {
    id: u32,
    name: String,
    role: String,
}

async fn fetch_user(id: u32) -> Result<(String, String), String> {
    users()
        .into_iter()
        .find(|u| u.id == id)
        .map(|u| (u.name, u.role))
        .ok_or_else(|| "not found".to_string())
}

fn users() -> Vec<User> {
    vec![
        User {
            id: 1,
            name: "Ada".into(),
            role: "Engineer".into(),
        },
        User {
            id: 2,
            name: "Bob".into(),
            role: "Designer".into(),
        },
        User {
            id: 3,
            name: "Clara".into(),
            role: "PM".into(),
        },
    ]
}

// --- pages -------------------------------------------------------------------

struct Home;
impl RouteTarget for Home {
    fn render(&self, _p: &Params, _o: VNode) -> VNode {
        rsx! {
            div {
                h1 { "Home" }
                p { "Welcome to the Ferrum SPA example." }
                ul {
                    li { a { href: "/users", "Users" } }
                    li { a { href: "/panel", "Panel (nested layout)" } }
                    li { a { href: "/about", "About" } }
                }
            }
        }
    }
}

struct UsersPage;
impl RouteTarget for UsersPage {
    fn render(&self, _p: &Params, _o: VNode) -> VNode {
        let client = client();
        match use_query(&client, "users", || async {
            Ok::<Vec<User>, String>(users())
        }) {
            QuerySnapshot::Loading => rsx! { p { "Loading users..." } },
            QuerySnapshot::Ok(list) => {
                let items: Vec<VNode> = list
                    .iter()
                    .map(|u| {
                        let href = format!("/users/{}", u.id);
                        let name = u.name.clone();
                        rsx! { li { a { href: href, "{name}" } } }
                    })
                    .collect();
                rsx! { div { h1 { "Users" } ul { {items} } } }
            }
            QuerySnapshot::Err(e) => rsx! { p { "Error: {e}" } },
        }
    }
}

struct UserPage;
impl RouteTarget for UserPage {
    fn render(&self, p: &Params, _o: VNode) -> VNode {
        let id: u32 = p.get("id").and_then(|s| s.parse().ok()).unwrap_or(0);
        let client = client();
        let key = format!("user:{id}");
        match use_query(&client, &key, move || fetch_user(id)) {
            QuerySnapshot::Loading => rsx! { p { "Loading user..." } },
            QuerySnapshot::Ok((name, role)) => {
                rsx! { div { h1 { "User {name}" } p { "Role: {role}" } } }
            }
            QuerySnapshot::Err(e) => rsx! { p { "Error: {e}" } },
        }
    }
}

struct About;
impl RouteTarget for About {
    fn render(&self, _p: &Params, _o: VNode) -> VNode {
        rsx! { div { h1 { "About" } p { "Ferrum: a React-style framework in Rust." } } }
    }
}

struct PanelLayout;
impl RouteTarget for PanelLayout {
    fn render(&self, _p: &Params, outlet: VNode) -> VNode {
        rsx! {
            div { class: "panel",
                nav { a { href: "/home", "Home" } }
                {outlet}
            }
        }
    }
}

struct PanelHome;
impl RouteTarget for PanelHome {
    fn render(&self, _p: &Params, _o: VNode) -> VNode {
        rsx! { h2 { "Panel Home" } }
    }
}

// --- client (thread-local, set at startup) -------------------------------------

thread_local! {
    static CLIENT: std::cell::RefCell<Option<Rc<Client>>> = const { std::cell::RefCell::new(None) };
}

fn client() -> Rc<Client> {
    CLIENT.with(|c| c.borrow().clone().expect("client not set"))
}

// --- app -----------------------------------------------------------------------

#[wasm_bindgen(start)]
pub fn start() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let root = document.get_element_by_id("app").expect("#app not found");

    let client = Client::new(Rc::new(wasm_bindgen_futures::spawn_local));
    CLIENT.with(|c| *c.borrow_mut() = Some(client));

    let history = BrowserHistory::new();
    let router = Router::new(
        Rules::new()
            .redirect("/", "/home")
            .route("/home", Home)
            .route("/users", UsersPage)
            .route("/users/:id", UserPage)
            .route("/about", About)
            .nest("/panel", PanelLayout, |r| r.route("", PanelHome)),
        history,
    );

    let app = AppHandle::new(root);
    let r2 = router.clone();
    app.render(move || {
        let m = r2.current();
        m.target.render(&m.params, m.outlet.clone())
    });
    // Re-render on navigation (pushState via links is intercepted by the
    // browser only for real <a>; this demo navigates via router.navigate).
    let a2 = app.clone();
    router.on_change(Rc::new(move || {
        a2.re_render();
    }));
    app.forget();
}
