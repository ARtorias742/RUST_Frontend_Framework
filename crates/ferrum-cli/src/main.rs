//! `ferrum` — the Ferrum CLI.
//!
//! Commands: `new`, `dev`, `build`, `check`, `doctor`, `release`.

use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");
    let rest = &args[1..];
    let code = match cmd {
        "new" => cmd_new(rest),
        "dev" => cmd_dev(rest),
        "build" => cmd_build(rest),
        "check" => cmd_check(rest),
        "doctor" => cmd_doctor(),
        "release" => cmd_release(rest),
        "add" => cmd_add(rest),
        "list" => cmd_add(&["--list".to_string()]),
        _ => {
            println!("ferrum — the Ferrum frontend framework CLI");
            println!("usage:");
            println!("  ferrum new <name>     scaffold a new app");
            println!("  ferrum dev [--port N] start the dev server (HMR + error overlay)");
            println!("  ferrum build          build the app for wasm (release)");
            println!("  ferrum check          type-check the app for wasm");
            println!("  ferrum doctor         check the toolchain");
            println!("  ferrum release [--version X]  verify + SBOM + size budget (1.0 gate)");
            println!(
                "  ferrum add <component>  copy a UI component into src/ui (--list, --dry-run)"
            );
            println!("  ferrum list             list installable UI components");
            0
        }
    };
    std::process::exit(code);
}

fn cmd_new(rest: &[String]) -> i32 {
    let Some(name) = rest.first() else {
        eprintln!("usage: ferrum new <name> [--template blank|spa|dashboard|ssr]");
        return 1;
    };
    let template = rest
        .windows(2)
        .find(|w| w[0] == "--template" || w[0] == "-t")
        .and_then(|w| w[1].parse::<Template>().ok())
        .unwrap_or(Template::Blank);
    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("error: '{}' already exists", name);
        return 1;
    }
    let write = |rel: &str, content: &str| -> std::io::Result<()> {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    };
    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
ferrum-core = "0.1"
ferrum-web = "0.1"
wasm-bindgen = "0.2"
web-sys = "0.3"
{}"#,
        template.extra_deps()
    );
    let index_html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Ferrum app</title>
</head>
<body>
  <div id="app"></div>
</body>
</html>
"#;
    let lib_rs = template.lib_rs();
    let style_css = "body { font-family: system-ui, sans-serif; margin: 2rem; }\n";
    let mut files = vec![
        ("Cargo.toml", cargo_toml.as_str()),
        ("index.html", index_html),
        ("src/lib.rs", lib_rs.as_str()),
        ("style.css", style_css),
    ];
    files.extend(template.extra_files());
    for (rel, content) in files {
        if let Err(e) = write(rel, content) {
            eprintln!("error writing {}: {}", rel, e);
            return 1;
        }
    }
    println!(
        "created {} ({} template; run `ferrum dev` inside)",
        name, template
    );
    0
}

/// Scaffold templates (`ferrum new --template <name>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Blank,
    Spa,
    Dashboard,
    Ssr,
}

impl std::str::FromStr for Template {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blank" => Ok(Template::Blank),
            "spa" => Ok(Template::Spa),
            "dashboard" => Ok(Template::Dashboard),
            "ssr" => Ok(Template::Ssr),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Template::Blank => "blank",
            Template::Spa => "spa",
            Template::Dashboard => "dashboard",
            Template::Ssr => "ssr",
        };
        f.write_str(s)
    }
}

impl Template {
    fn extra_deps(&self) -> &'static str {
        match self {
            Template::Blank => "",
            Template::Spa => "ferrum-router = \"0.1\"\n",
            Template::Dashboard => "ferrum-components = \"0.1\"\nferrum-styling = \"0.1\"\n",
            Template::Ssr => "ferrum-ssr = \"0.1\"\nferrum-styling = \"0.1\"\n",
        }
    }

    fn lib_rs(&self) -> String {
        match self {
            Template::Blank => r#"use ferrum_core::{component, rsx, use_state};
use ferrum_web::AppHandle;
use wasm_bindgen::prelude::*;

#[component]
fn Counter() -> ferrum_core::VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        div {
            h1 { "Ferrum" }
            button { onclick: move |_| set.update(|c| c + 1), "Count: {count}" }
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let app = AppHandle::new(root);
    app.render(|| rsx! { Counter {} });
    app.forget();
}
"#
            .to_string(),
            Template::Spa => r#"use ferrum_core::{rsx, VNode};
use std::rc::Rc;
use ferrum_router::{Params, RouteTarget, Router, Rules};
use ferrum_web::{AppHandle, BrowserHistory};
use wasm_bindgen::prelude::*;

struct Home;
impl RouteTarget for Home {
    fn render(&self, _p: &Params, _o: VNode) -> VNode {
        rsx! { div { h1 { "Home" } p { "Welcome." } a { href: "/about", "About" } } }
    }
}

struct About;
impl RouteTarget for About {
    fn render(&self, _p: &Params, _o: VNode) -> VNode {
        rsx! { div { h1 { "About" } a { href: "/", "Home" } } }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let router = Router::new(
        Rules::new()
            .route("/", Home)
            .route("/about", About),
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
    app.forget();
}
"#
            .to_string(),
            Template::Dashboard => {
                r#"use ferrum_components::{Badge, Button, Card, Progress, Table, ThemeRoot};
use ferrum_core::{rsx, VNode};
use std::rc::Rc;
use ferrum_styling::Theme;
use ferrum_web::AppHandle;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let app = AppHandle::new(root);
    app.render(|| {
        rsx! {
            ThemeRoot { theme: Theme::light(),
                Card { title: "Dashboard",
                    Progress { value: 60, max: 100 }
                    Table { columns: vec!["Name".to_string(), "Role".to_string()],
                            rows: vec![vec!["Ada".to_string(), "Engineer".to_string()]] }
                    Badge { label: "live" }
                    Button { variant: "primary", on_click: None, "Refresh" }
                }
            }
        }
    });
    app.forget();
}
"#
                .to_string()
            }
            Template::Ssr => r#"use ferrum_core::{component, rsx, use_state, VNode};
use ferrum_web::AppHandle;
use wasm_bindgen::prelude::*;

#[component]
pub fn Counter() -> VNode {
    let (count, set) = use_state(|| 0i32);
    rsx! {
        div {
            h1 { "Ferrum SSR" }
            button { onclick: move |_| set.update(|c| c + 1), "Count: {count}" }
        }
    }
}

/// Server-side HTML for the same component (used by the host).
pub fn render_html() -> String {
    ferrum_ssr::render_document("Ferrum SSR", &ferrum_ssr::render_to_string(&rsx! { Counter {} }))
}

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let app = AppHandle::new(root);
    app.render(|| rsx! { Counter {} });
    app.forget();
}
"#
            .to_string(),
        }
    }

    fn extra_files(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            Template::Dashboard => vec![(
                "src/stat.rs",
                r#"// Extra widgets can live in their own module.
"#,
            )],
            _ => Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// ferrum add — source-distributed UI components
// ---------------------------------------------------------------------------

struct UiTemplate {
    name: &'static str,
    version: &'static str,
    category: &'static str,
    deps: &'static [&'static str],
    files: &'static [(&'static str, &'static str)],
}

const BUTTON_RS: &str = r#"use ferrum_core::{component, rsx, EventHandler, VNode};
use ferrum_styling::css;

/// A button. `variant`: "primary" (default) or "ghost".
#[component(default)]
pub fn Button(
    variant: String,
    disabled: bool,
    on_click: Option<EventHandler>,
    children: Vec<VNode>,
) -> VNode {
    let base = css! {
        all: unset;
        box-sizing: border-box;
        cursor: pointer;
        padding: 8px 16px;
        border-radius: var(--radius);
        font: inherit;
        text-align: center;
    };
    let variant_class = if variant.as_str() == "ghost" {
        css! {
            background: transparent;
            color: var(--fg);
            border: 1px solid var(--border);
        }
    } else {
        css! {
            background: var(--accent);
            color: var(--accent-fg);
            border: 1px solid var(--accent);
        }
    };
    let class = format!("{} {}", base.name(), variant_class.name());
    match on_click {
        Some(handler) if !disabled => {
            let h = handler.clone();
            rsx! {
                button { class: class, onclick: move |e: &ferrum_core::FerrumEvent| h(e), {children} }
            }
        }
        _ => rsx! { button { class: class, disabled: disabled, {children} } },
    }
}
"#;

const DIALOG_RS: &str = r#"use ferrum_core::{component, rsx, EventHandler, VNode};
use ferrum_styling::css;
use ferrum_ui_core::FocusScope;

/// A modal dialog. Render nothing when `open` is false.
#[component]
pub fn Dialog(
    open: bool,
    title: String,
    on_close: Option<EventHandler>,
    children: Vec<VNode>,
) -> VNode {
    if !open {
        return VNode::Empty;
    }
    // The scope is the headless focus-trap bookkeeping; the host wires real
    // focus ids to it.
    let scope = FocusScope::new(0, "dialog");
    let _first = scope.first(&[]);
    let backdrop = css! {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    };
    let dialog = css! {
        background: var(--bg);
        color: var(--fg);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        padding: 20px;
        min-width: 320px;
        max-width: 90vw;
    };
    let head = css! { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
    let close = css! { all: unset; cursor: pointer; font-size: 1.2rem; line-height: 1; }
    let close_node = match on_close {
        Some(handler) => {
            let h = handler.clone();
            rsx! {
                button { class: close.name(), aria-label: "Close", onclick: move |e: &ferrum_core::FerrumEvent| h(e), "×" }
            }
        }
        None => rsx! { span { class: close.name(), aria-hidden: true, "×" } },
    };
    rsx! {
        div { class: backdrop.name(),
            div { class: dialog.name(), role: "dialog", aria-modal: true, aria-label: title,
                div { class: head.name(), h2 { "{title}" } {close_node} }
                {children}
            }
        }
    }
}
"#;

const MENU_RS: &str = r#"use ferrum_core::{component, rsx, EventHandler, VNode};
use ferrum_styling::css;
use ferrum_ui_core::RovingTabIndex;

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub disabled: bool,
    pub on_select: Option<EventHandler>,
}

#[component]
pub fn Menu(open: bool, active: usize, items: Vec<MenuItem>, children: Vec<VNode>) -> VNode {
    let open = *open;
    let active = *active;
    let wrap = css! { position: relative; display: inline-block; };
    let list = css! {
        position: absolute; top: 100%; inset-inline-start: 0; min-width: 160px;
        background: var(--bg); border: 1px solid var(--border);
        border-radius: var(--radius); padding: 4px; z-index: 10;
    };
    let item = css! {
        all: unset; box-sizing: border-box; display: block; width: 100%;
        cursor: pointer; padding: 6px 10px; border-radius: 4px;
    };
    let roving = RovingTabIndex::new(active);
    let entries: Vec<VNode> = items.iter().enumerate().map(|(i, entry)| {
        let tabindex = roving.tab_index(i) as i32;
        match &entry.on_select {
            Some(handler) if !entry.disabled => {
                let h = handler.clone();
                rsx! { button { class: item.name(), role: "menuitem", tabindex: tabindex,
                    onclick: move |e: &ferrum_core::FerrumEvent| h(e), "{entry.label}" } }
            }
            _ => rsx! { button { class: item.name(), role: "menuitem", tabindex: tabindex,
                aria-disabled: entry.disabled, "{entry.label}" } },
        }
    }).collect();
    let popup = if open { rsx! { div { class: list.name(), role: "menu", {entries} } } } else { VNode::Empty };
    rsx! { div { class: wrap.name(), {children} {popup} } }
}
"#;

const TABS_RS: &str = r#"use ferrum_core::{component, rsx, EventHandler, VNode};
use ferrum_styling::css;
use ferrum_ui_core::RovingTabIndex;

#[derive(Clone)]
pub struct TabItem { pub label: String, pub content: VNode }

#[component]
pub fn Tabs(orientation: String, active: usize, on_change: Option<EventHandler>, items: Vec<TabItem>) -> VNode {
    let active = *active;
    let bar = css! { display: flex; gap: 4px; border-bottom: 1px solid var(--border); };
    let tab = css! { all: unset; cursor: pointer; padding: 8px 12px; border-bottom: 2px solid transparent; };
    let active_tab = css! { border-bottom-color: var(--accent); font-weight: 600; };
    let panel = css! { padding: 12px 0; };
    let roving = RovingTabIndex::new(active);
    let buttons: Vec<VNode> = items.iter().enumerate().map(|(i, item)| {
        let class = if i == active { format!("{} {}", tab.name(), active_tab.name()) } else { tab.name().to_string() };
        let tabindex = roving.tab_index(i) as i32;
        match &on_change {
            Some(h) => { let idx = i; rsx! { button { class: class, role: "tab", aria-selected: i == active,
                tabindex: tabindex, onclick: move |e: &ferrum_core::FerrumEvent| {
                    h(&ferrum_core::FerrumEvent::new("tab", Box::new(idx))); let _ = e;
                }, "{item.label}" } } }
            None => rsx! { button { class: class, role: "tab", aria-selected: i == active, tabindex: tabindex, "{item.label}" } },
        }
    }).collect();
    let content = items.get(active).map(|i| i.content.clone()).unwrap_or(VNode::Empty);
    let aria_orientation = if orientation.as_str() == "vertical" { "vertical" } else { "horizontal" };
    rsx! { div {
        div { class: bar.name(), role: "tablist", aria-orientation: aria_orientation, {buttons} }
        div { class: panel.name(), role: "tabpanel", {content} }
    } }
}
"#;

const TOAST_RS: &str = r#"use ferrum_core::{component, rsx, EventHandler, VNode};
use ferrum_styling::css;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastLevel { #[default] Info, Success, Warning, Error }

#[derive(Clone, Debug)]
pub struct Toast { pub id: u64, pub level: ToastLevel, pub title: String, pub description: String }

#[component]
pub fn Toaster(toasts: Vec<Toast>, on_dismiss: Option<EventHandler>) -> VNode {
    let region = css! { position: fixed; inset-inline-end: 16px; bottom: 16px; display: flex; flex-direction: column; gap: 8px; z-index: 2000; };
    let toast = css! { min-width: 240px; padding: 10px 14px; border-radius: var(--radius); background: var(--fg); color: var(--bg); };
    let rows: Vec<VNode> = toasts.iter().map(|t| {
        let id = t.id;
        let role = if t.level == ToastLevel::Error { "alert" } else { "status" };
        let close = match &on_dismiss {
            Some(h) => { let h2 = h.clone(); rsx! { button { aria-label: "Dismiss",
                onclick: move |e: &ferrum_core::FerrumEvent| { h2(&ferrum_core::FerrumEvent::new("dismiss", Box::new(id))); let _ = e; }, "×" } } }
            None => rsx! { span { aria-hidden: true, "×" } },
        };
        rsx! { div { class: toast.name(), role: role, div { "{t.title}" } div { "{t.description}" } {close} } }
    }).collect();
    rsx! { div { class: region.name(), aria-live: "polite", {rows} } }
}
"#;

const TEMPLATES: &[UiTemplate] = &[
    UiTemplate {
        name: "button",
        version: "0.1.0",
        category: "action",
        deps: &["ferrum-core", "ferrum-styling"],
        files: &[("button.rs", BUTTON_RS)],
    },
    UiTemplate {
        name: "dialog",
        version: "0.1.0",
        category: "overlay",
        deps: &["ferrum-core", "ferrum-styling", "ferrum-ui-core"],
        files: &[("dialog.rs", DIALOG_RS)],
    },
    UiTemplate {
        name: "menu",
        version: "0.1.0",
        category: "navigation",
        deps: &["ferrum-core", "ferrum-styling", "ferrum-ui-core"],
        files: &[("menu.rs", MENU_RS)],
    },
    UiTemplate {
        name: "tabs",
        version: "0.1.0",
        category: "navigation",
        deps: &["ferrum-core", "ferrum-styling", "ferrum-ui-core"],
        files: &[("tabs.rs", TABS_RS)],
    },
    UiTemplate {
        name: "toast",
        version: "0.1.0",
        category: "feedback",
        deps: &["ferrum-core", "ferrum-styling"],
        files: &[("toast.rs", TOAST_RS)],
    },
];

fn template_registry() -> ferrum_registry::TemplateRegistry {
    let mut registry = ferrum_registry::TemplateRegistry::new();
    for t in TEMPLATES {
        let files: Vec<&str> = t.files.iter().map(|(name, _)| *name).collect();
        registry.register(
            t.name,
            t.version.parse().unwrap(),
            t.category,
            &files,
            t.deps,
        );
    }
    registry
}

fn cmd_add(rest: &[String]) -> i32 {
    if rest.iter().any(|a| a == "--list") {
        println!("Ferrum UI components (source-distributed):");
        for entry in template_registry().iter() {
            println!(
                "  {:<10} v{}  {:<8} deps: {}",
                entry.name,
                entry.version,
                entry.category,
                entry.deps.join(", ")
            );
        }
        return 0;
    }

    let Some(name) = rest.first().filter(|a| !a.starts_with("--")) else {
        eprintln!("usage: ferrum add <component> [--dir DIR] [--dry-run] [--force]");
        return 1;
    };
    let Some(template) = TEMPLATES.iter().find(|t| t.name == name) else {
        eprintln!("error: unknown component '{name}' (try `ferrum list`)");
        return 1;
    };

    let dir = rest
        .windows(2)
        .find(|w| w[0] == "--dir")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "src/ui".to_string());
    let dry_run = rest.iter().any(|a| a == "--dry-run");
    let diff = rest.iter().any(|a| a == "--diff");
    let force = rest.iter().any(|a| a == "--force" || a == "--update");
    let target = Path::new(&dir);

    if diff {
        for (file, contents) in template.files {
            let path = target.join(file);
            let old = std::fs::read_to_string(&path).unwrap_or_default();
            if old == *contents {
                println!("  {} unchanged", path.display());
                continue;
            }
            println!("--- a/{}", path.display());
            println!("+++ b/{}", path.display());
            for line in contents.lines() {
                if !old.lines().any(|old_line| old_line == line) {
                    println!("+{line}");
                }
            }
            for line in old.lines() {
                if !contents.lines().any(|new_line| new_line == line) {
                    println!("-{line}");
                }
            }
        }
        return 0;
    }

    for (file, contents) in template.files {
        let path = target.join(file);
        if path.exists() && !force {
            eprintln!(
                "error: {} already exists (use --force or --update)",
                path.display()
            );
            return 1;
        }
        if dry_run {
            println!("--- {}", path.display());
            print!("{contents}");
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("error: create {}: {e}", parent.display());
                std::process::exit(1);
            });
        }
        std::fs::write(&path, contents).unwrap_or_else(|e| {
            eprintln!("error: write {}: {e}", path.display());
            std::process::exit(1);
        });
        println!("wrote {}", path.display());
    }

    if !dry_run {
        let mod_path = target.join("mod.rs");
        let entry = format!("pub mod {name};");
        let existing = std::fs::read_to_string(&mod_path).unwrap_or_default();
        if !existing.contains(&entry) {
            let mut out = existing;
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&entry);
            out.push('\n');
            std::fs::write(&mod_path, out).unwrap_or_else(|e| {
                eprintln!("error: write {}: {e}", mod_path.display());
                std::process::exit(1);
            });
        }

        let manifest = target.join("manifest.toml");
        let mut body = std::fs::read_to_string(&manifest).unwrap_or_default();
        let section = format!("[components.{name}]");
        if !body.contains(&section) {
            if !body.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&format!(
                "{section}\nversion = \"{}\"\nsource = \"ferrum-ui\"\nfiles = [{}]\n",
                template.version,
                template
                    .files
                    .iter()
                    .map(|(f, _)| format!("\"{f}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            std::fs::write(&manifest, body).unwrap_or_else(|e| {
                eprintln!("error: write {}: {e}", manifest.display());
                std::process::exit(1);
            });
        }
        println!("updated {}", target.join("mod.rs").display());
    }
    0
}

fn cmd_dev(rest: &[String]) -> i32 {
    let port = rest
        .windows(2)
        .find(|w| w[0] == "--port")
        .and_then(|w| w[1].parse::<u16>().ok())
        .unwrap_or(8080);
    let root = std::env::current_dir().unwrap_or_else(|_| ".".into());
    match ferrum_dev::start(&root, port) {
        Ok(handle) => {
            println!("  ferrum dev");
            println!(
                "  ➜ Local:   http://localhost:{}/  (HMR: css hot-swap, rebuild+reload)",
                handle.port
            );
            println!("  watching {} — Ctrl+C to stop", root.display());
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            1
        }
    }
}

fn cmd_build(rest: &[String]) -> i32 {
    let budget_kb = rest
        .windows(2)
        .find(|w| w[0] == "--size-budget")
        .and_then(|w| w[1].parse::<u64>().ok());
    let code = run(
        "cargo",
        &["build", "--release", "--target", "wasm32-unknown-unknown"],
    );
    if code != 0 {
        return code;
    }
    match budget_kb {
        Some(kb) => check_size_budget(kb),
        None => 0,
    }
}

/// The 1.0 release gate: version consistency, tests, size budget, SBOM.
fn cmd_release(rest: &[String]) -> i32 {
    let version = rest
        .windows(2)
        .find(|w| w[0] == "--version")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    if rest.iter().any(|a| a == "--sbom-only") {
        return match generate_sbom(&version) {
            Ok(path) => {
                println!("✓ SBOM written to {}", path.display());
                0
            }
            Err(e) => {
                eprintln!("error: SBOM generation failed: {e}");
                1
            }
        };
    }

    // 1. The workspace version must match the release version.
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap_or_default();
    let workspace_version = manifest
        .lines()
        .find(|l| l.trim().starts_with("version ="))
        .and_then(|l| l.split('"').nth(1))
        .unwrap_or("");
    if workspace_version != version {
        eprintln!("error: release version {version} != workspace version {workspace_version}");
        return 1;
    }
    if version.parse::<ferrum_registry::Version>().is_err() {
        eprintln!("error: release version {version} is not MAJOR.MINOR.PATCH");
        return 1;
    }
    println!("✓ version {version} matches the workspace");

    // 2. Full test suite.
    if run("cargo", &["test", "--workspace"]) != 0 {
        eprintln!("error: tests failed");
        return 1;
    }
    println!("✓ workspace tests green");

    // 3. Size budget (gzip is checked by scripts/check-size.sh; here we check
    //    the raw wasm against a conservative 1 MB).
    if check_size_budget(1024) != 0 {
        return 1;
    }

    // 4. SBOM.
    match generate_sbom(&version) {
        Ok(path) => println!("✓ SBOM written to {}", path.display()),
        Err(e) => {
            eprintln!("error: SBOM generation failed: {e}");
            return 1;
        }
    }

    println!("release {version} gates passed");
    0
}

fn check_size_budget(budget_kb: u64) -> i32 {
    let dir = Path::new("target/wasm32-unknown-unknown/release");
    let mut largest: Option<(u64, std::path::PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("wasm") {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                if largest.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
                    largest = Some((size, path));
                }
            }
        }
    }
    match largest {
        Some((bytes, path)) => {
            let kb = bytes / 1024;
            println!("  wasm size: {} KB ({})", kb, path.display());
            if kb > budget_kb {
                eprintln!("error: wasm size {kb} KB exceeds the {budget_kb} KB budget");
                return 1;
            }
            0
        }
        None => {
            eprintln!("error: no wasm artifact found (run `ferrum build` first)");
            1
        }
    }
}

/// Write a minimal CycloneDX 1.5 SBOM for the workspace crates.
fn generate_sbom(version: &str) -> std::io::Result<std::path::PathBuf> {
    let dir = Path::new("target/sbom");
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("ferrum-{version}.cdx.json"));

    let mut components = String::new();
    let mut first = true;
    if let Ok(entries) = std::fs::read_dir("crates") {
        for entry in entries.flatten() {
            let manifest = entry.path().join("Cargo.toml");
            let Ok(text) = std::fs::read_to_string(&manifest) else {
                continue;
            };
            let name = text
                .lines()
                .find(|l| l.trim().starts_with("name ="))
                .and_then(|l| l.split('"').nth(1))
                .unwrap_or("");
            let ver = text
                .lines()
                .find(|l| l.trim().starts_with("version ="))
                .and_then(|l| l.split('"').nth(1))
                .unwrap_or(version);
            if name.is_empty() {
                continue;
            }
            if !first {
                components.push(',');
            }
            first = false;
            components.push_str(&format!(
                "{{\"type\":\"library\",\"name\":\"{name}\",\"version\":\"{ver}\"}}"
            ));
        }
    }
    let json = format!(
        r#"{{"bomFormat":"CycloneDX","specVersion":"1.5","version":1,"metadata":{{"component":{{"type":"application","name":"ferrum","version":"{version}"}}}},"components":[{components}]}}
"#
    );
    std::fs::write(&path, json)?;
    Ok(path)
}

fn cmd_check(_rest: &[String]) -> i32 {
    run("cargo", &["check", "--target", "wasm32-unknown-unknown"])
}

fn run(program: &str, args: &[&str]) -> i32 {
    let status = Command::new(program).args(args).status();
    match status {
        Ok(s) if s.success() => 0,
        Ok(_) => 1,
        Err(e) => {
            eprintln!("error: failed to run {}: {}", program, e);
            1
        }
    }
}

fn cmd_doctor() -> i32 {
    let mut ok = true;
    let checks: &[(&str, &[&str])] = &[
        ("rustc", &["--version"]),
        ("cargo", &["--version"]),
        ("rustup", &["target", "list", "--installed"]),
    ];
    for (program, args) in checks {
        match Command::new(program).args(*args).output() {
            Ok(o) if o.status.success() => {
                println!(
                    "✓ {} {}",
                    program,
                    String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .next()
                        .unwrap_or("")
                )
            }
            _ => {
                println!("✗ {} not found", program);
                ok = false;
            }
        }
    }
    let has_wasm = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("wasm32-unknown-unknown"))
        .unwrap_or(false);
    if has_wasm {
        println!("✓ wasm32-unknown-unknown target installed");
    } else {
        println!("✗ wasm32 target missing (rustup target add wasm32-unknown-unknown)");
        ok = false;
    }
    // No-eval lint: the CSP gate forbids `eval` / `new Function`.
    for file in ["index.html", "style.css"] {
        if let Ok(text) = std::fs::read_to_string(file) {
            if ferrum_security::uses_eval(&text) {
                println!("✗ {file} contains eval/new Function (CSP forbids it)");
                ok = false;
            }
        }
    }
    if !ok {
        return 1;
    }
    println!("all good — run `ferrum dev`");
    0
}
