//! Phase 4 gate (router): static routes, params, nested layouts, redirects,
//! deep links, navigation.

use ferrum_core::{rsx, VNode};
use ferrum_router::{ChunkInfo, History, MemoryHistory, Params, RouteTarget, Router, Rules};
use ferrum_testing::TestRenderer;
use std::rc::Rc;

struct Home;
impl RouteTarget for Home {
    fn render(&self, _params: &Params, _outlet: VNode) -> VNode {
        rsx! { h1 { "Home" } }
    }
}

struct Users;
impl RouteTarget for Users {
    fn render(&self, _params: &Params, _outlet: VNode) -> VNode {
        rsx! { h1 { "Users" } }
    }
}

struct UserDetail;
impl RouteTarget for UserDetail {
    fn render(&self, params: &Params, _outlet: VNode) -> VNode {
        let id = params.get("id").unwrap_or("?").to_string();
        rsx! { h1 { "User {id}" } }
    }
}

struct About;
impl RouteTarget for About {
    fn render(&self, _params: &Params, _outlet: VNode) -> VNode {
        rsx! { h1 { "About" } }
    }
}

struct PanelLayout;
impl RouteTarget for PanelLayout {
    fn render(&self, _params: &Params, outlet: VNode) -> VNode {
        rsx! { div { class: "panel", h2 { "Panel" }, {outlet} } }
    }
}

struct PanelHome;
impl RouteTarget for PanelHome {
    fn render(&self, _params: &Params, _outlet: VNode) -> VNode {
        rsx! { h3 { "Panel Home" } }
    }
}

fn rules() -> Rules {
    Rules::new()
        .route("/", Home)
        .route("/users", Users)
        .route("/users/:id", UserDetail)
        .route("/about", About)
        .redirect("/legacy", "/users")
        .nest("/panel", PanelLayout, |r| {
            r.route("", PanelHome).route("info", About)
        })
}

#[test]
fn static_route_matches() {
    let router = Router::new(rules(), Rc::new(MemoryHistory::at("/users")));
    let m = router.current();
    let r = TestRenderer::new();
    r.render(move || m.target.render(&m.params, m.outlet.clone()));
    assert_eq!(r.text(), "Users");
}

#[test]
fn param_route_captures() {
    let router = Router::new(rules(), Rc::new(MemoryHistory::at("/users/42")));
    let m = router.current();
    assert_eq!(m.params.get("id"), Some("42"));
    let r = TestRenderer::new();
    r.render(move || m.target.render(&m.params, m.outlet.clone()));
    assert_eq!(r.text(), "User 42");
}

#[test]
fn nested_layout_renders_outlet() {
    let router = Router::new(rules(), Rc::new(MemoryHistory::at("/panel")));
    let m = router.current();
    let r = TestRenderer::new();
    r.render(move || m.target.render(&m.params, m.outlet.clone()));
    assert_eq!(r.text(), "PanelPanel Home");
    assert_eq!(r.find_all("div").len(), 1);
}

#[test]
fn nested_layout_with_child() {
    let router = Router::new(rules(), Rc::new(MemoryHistory::at("/panel/info")));
    let m = router.current();
    let r = TestRenderer::new();
    r.render(move || m.target.render(&m.params, m.outlet.clone()));
    assert_eq!(r.text(), "PanelAbout");
}

#[test]
fn redirect_is_followed() {
    let router = Router::new(rules(), Rc::new(MemoryHistory::at("/legacy")));
    let m = router.current();
    let r = TestRenderer::new();
    r.render(move || m.target.render(&m.params, m.outlet.clone()));
    assert_eq!(r.text(), "Users");
}

#[test]
fn unknown_path_is_404() {
    let router = Router::new(rules(), Rc::new(MemoryHistory::at("/nope")));
    let m = router.current();
    let r = TestRenderer::new();
    r.render(move || m.target.render(&m.params, m.outlet.clone()));
    assert!(r.text().contains("404"));
}

#[test]
fn navigate_and_deep_link() {
    let history = Rc::new(MemoryHistory::at("/"));
    let router = Router::new(rules(), history.clone());
    let r = TestRenderer::new();
    let router2 = router.clone();
    r.render(move || {
        let m = router2.current();
        m.target.render(&m.params, m.outlet.clone())
    });
    assert_eq!(r.text(), "Home");

    // Navigate via a link click -> router.navigate (push + notify).
    router.navigate("/about");
    // The router notifies subscribers; wire on_change to re_render.
    assert_eq!(router.current().params.get("id"), None);
    let r2 = TestRenderer::new();
    let router3 = router.clone();
    r2.render(move || {
        let m = router3.current();
        m.target.render(&m.params, m.outlet.clone())
    });
    assert_eq!(r2.text(), "About");
}

#[test]
fn on_change_notifies_on_navigation() {
    use std::cell::Cell;
    let calls = Rc::new(Cell::new(0));
    let history = Rc::new(MemoryHistory::at("/"));
    let router = Router::new(rules(), history);
    let c2 = calls.clone();
    router.on_change(Rc::new(move || {
        c2.set(c2.get() + 1);
    }));
    router.navigate("/users");
    router.navigate("/users/7");
    assert_eq!(calls.get(), 2, "subscriber called once per navigation");
}

#[test]
fn back_navigates() {
    let history = Rc::new(MemoryHistory::at("/"));
    history.push("/users");
    assert_eq!(history.current(), "/users");
    history.back();
    assert_eq!(history.current(), "/");
}
struct Chunked;
impl RouteTarget for Chunked {
    fn render(&self, _params: &Params, _outlet: VNode) -> VNode {
        rsx! { h1 { "Chunked" } }
    }
    fn chunk(&self) -> Option<ChunkInfo> {
        Some(ChunkInfo::new("route-chunked", 100 * 1024))
    }
}

#[test]
fn route_chunk_metadata_and_budget() {
    let router = Router::new(
        Rules::new().route("/chunked", Chunked),
        Rc::new(MemoryHistory::at("/chunked")),
    );
    let m = router.current();
    let chunk = m.chunk.expect("chunk metadata");
    assert_eq!(chunk.name, "route-chunked");
    assert_eq!(chunk.budget_bytes, 100 * 1024);
    assert!(chunk.check(80 * 1024).is_ok());
    assert!(chunk.check(120 * 1024).is_err());
}
