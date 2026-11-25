//! # ferrum-components
//!
//! The Ferrum component library: 30 headless-styled widgets built with the
//! public `#[component]` + `rsx!` API and scoped `css!` classes.
//!
//! Every widget is a plain component styled with `var(--token)` references,
//! so themes (dark mode, RTL) switch without recompiling, and every widget
//! renders identically on the web, headless and native backends (same attrs,
//! same DOM shape).
//!
//! The [`CATALOG`] constant is the machine-readable inventory used by the
//! docs generator and the registry (`ferrum-registry`).
//!
//! ## Conventions
//!
//! - Controlled components take their value as a prop plus an optional
//!   `on_*` [`EventHandler`]; they keep no hidden state.
//! - `children` is always the trailing block: `Card { title: "Hi" } { ... }`.
//! - Interactive widgets set ARIA roles/attributes (`role="dialog"`,
//!   `aria-checked`, `role="progressbar"`, ...).

use ferrum_core::{component, rsx, use_id, EventHandler, VNode};
use ferrum_styling::{css, theme_context, Theme};

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

/// One entry in the component inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentMeta {
    pub name: &'static str,
    pub category: &'static str,
    pub since: &'static str,
}

/// All 30 components, in documentation order.
pub const CATALOG: &[ComponentMeta] = &[
    ComponentMeta {
        name: "ThemeRoot",
        category: "layout",
        since: "0.1",
    },
    ComponentMeta {
        name: "Button",
        category: "action",
        since: "0.1",
    },
    ComponentMeta {
        name: "Card",
        category: "layout",
        since: "0.1",
    },
    ComponentMeta {
        name: "Badge",
        category: "data-display",
        since: "0.1",
    },
    ComponentMeta {
        name: "Input",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Textarea",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Select",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Checkbox",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Radio",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Switch",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Label",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "FormField",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Slider",
        category: "form",
        since: "1.0",
    },
    ComponentMeta {
        name: "Modal",
        category: "overlay",
        since: "1.0",
    },
    ComponentMeta {
        name: "Tooltip",
        category: "overlay",
        since: "1.0",
    },
    ComponentMeta {
        name: "Tabs",
        category: "navigation",
        since: "1.0",
    },
    ComponentMeta {
        name: "Accordion",
        category: "data-display",
        since: "1.0",
    },
    ComponentMeta {
        name: "Menu",
        category: "navigation",
        since: "1.0",
    },
    ComponentMeta {
        name: "Alert",
        category: "feedback",
        since: "1.0",
    },
    ComponentMeta {
        name: "Toast",
        category: "feedback",
        since: "1.0",
    },
    ComponentMeta {
        name: "Progress",
        category: "feedback",
        since: "1.0",
    },
    ComponentMeta {
        name: "Spinner",
        category: "feedback",
        since: "1.0",
    },
    ComponentMeta {
        name: "Skeleton",
        category: "feedback",
        since: "1.0",
    },
    ComponentMeta {
        name: "Avatar",
        category: "data-display",
        since: "1.0",
    },
    ComponentMeta {
        name: "Table",
        category: "data-display",
        since: "1.0",
    },
    ComponentMeta {
        name: "List",
        category: "data-display",
        since: "1.0",
    },
    ComponentMeta {
        name: "Pagination",
        category: "navigation",
        since: "1.0",
    },
    ComponentMeta {
        name: "Breadcrumb",
        category: "navigation",
        since: "1.0",
    },
    ComponentMeta {
        name: "Divider",
        category: "layout",
        since: "1.0",
    },
    ComponentMeta {
        name: "EmptyState",
        category: "layout",
        since: "1.0",
    },
];

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

/// Provides theme tokens (as CSS custom properties) + `dir` to the subtree.
#[component]
pub fn ThemeRoot(theme: Theme, children: Vec<VNode>) -> VNode {
    let style = theme.style_string();
    let dir = theme.dir;
    let inner = theme_context().provider(theme.clone(), children.clone());
    let VNode::Provider(p) = inner else {
        unreachable!("provider always returns VNode::Provider")
    };
    VNode::Element(ferrum_core::VElement {
        tag: "div".to_string(),
        attrs: vec![
            ("style".to_string(), ferrum_core::AttrValue::Str(style)),
            (
                "dir".to_string(),
                ferrum_core::AttrValue::Str(dir.to_string()),
            ),
        ],
        listeners: Vec::new(),
        // Keep the Provider wrapper so renderers push the context value.
        children: vec![VNode::Provider(p)],
        key: None,
    })
}

/// A card with a title and body.
#[component]
pub fn Card(title: String, children: Vec<VNode>) -> VNode {
    let card = css! {
        background: var(--bg);
        color: var(--fg);
        border: 1px solid var(--muted);
        border-radius: var(--radius);
        padding: 16px;
    };
    let title_cls = css! {
        margin: 0 0 8px 0;
        font-size: 1.25rem;
    };
    rsx! {
        div { class: card.name(),
            h2 { class: title_cls.name(), "{title}" }
            {children}
        }
    }
}

/// A vertical rule with an optional centered label.
#[component]
pub fn Divider(label: String) -> VNode {
    let rule = css! {
        display: flex;
        align-items: center;
        gap: 12px;
        color: var(--muted);
        font-size: 0.85rem;
        margin: 12px 0;
    };
    let line = css! { flex: 1; height: 1px; background: var(--muted); };
    if label.is_empty() {
        rsx! { div { class: rule.name(), div { class: line.name() } } }
    } else {
        rsx! {
            div { class: rule.name(),
                div { class: line.name() }
                span { "{label}" }
                div { class: line.name() }
            }
        }
    }
}

/// An empty-state placeholder with an optional action.
#[component]
pub fn EmptyState(title: String, description: String, children: Vec<VNode>) -> VNode {
    let box_ = css! {
        text-align: center;
        color: var(--muted);
        padding: 32px 16px;
        border: 1px dashed var(--muted);
        border-radius: var(--radius);
    };
    let title_cls = css! { color: var(--fg); font-weight: 600; margin: 0 0 4px 0; };
    rsx! {
        div { class: box_.name(),
            p { class: title_cls.name(), "{title}" }
            p { "{description}" }
            {children}
        }
    }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/// A button. `variant`: "primary" (default) or "ghost".
#[component]
pub fn Button(variant: String, on_click: Option<EventHandler>, children: Vec<VNode>) -> VNode {
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
            border: 1px solid var(--muted);
        }
    } else {
        css! {
            background: var(--accent);
            color: var(--bg);
            border: 1px solid var(--accent);
        }
    };
    let class = format!("{} {}", base.name(), variant_class.name());
    match on_click {
        Some(handler) => {
            let h2 = handler.clone();
            rsx! {
                button { class: class, onclick: move |e: &ferrum_core::FerrumEvent| h2(e), {children} }
            }
        }
        None => rsx! {
            button { class: class, {children} }
        },
    }
}

// ---------------------------------------------------------------------------
// Data display
// ---------------------------------------------------------------------------

/// A small inline badge.
#[component]
pub fn Badge(label: String) -> VNode {
    let badge = css! {
        display: inline-block;
        padding: 2px 8px;
        border-radius: 999px;
        background: var(--accent);
        color: var(--bg);
        font-size: 0.85rem;
    };
    rsx! {
        span { class: badge.name(), "{label}" }
    }
}

/// A user avatar: initials, or an image when `src` is set.
#[component]
pub fn Avatar(name: String, src: String, size: String) -> VNode {
    let box_ = css! {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border-radius: 50%;
        background: var(--muted);
        color: var(--bg);
        font-weight: 600;
        overflow: hidden;
    };
    let size_style = if size.is_empty() {
        "40px".to_string()
    } else {
        size.clone()
    };
    let style = format!("width: {}; height: {};", size_style, size_style);
    if src.is_empty() {
        let initials: String = name
            .split_whitespace()
            .take(2)
            .map(|w| w.chars().next().unwrap_or('?'))
            .collect();
        rsx! { span { class: box_.name(), style: style, "{initials}" } }
    } else {
        rsx! { span { class: box_.name(), style: style, img { src: src, alt: "{name}" } } }
    }
}

/// A data table. `columns` are headers; `rows` are cell values.
#[component]
pub fn Table(columns: Vec<String>, rows: Vec<Vec<String>>) -> VNode {
    let table = css! {
        border-collapse: collapse;
        width: 100%;
        font-size: 0.95rem;
    };
    let cell = css! {
        border-bottom: 1px solid var(--muted);
        padding: 6px 10px;
        text-align: start;
    };
    let head: Vec<VNode> = columns
        .iter()
        .map(|c| rsx! { th { class: cell.name(), "{c}" } })
        .collect();
    let body: Vec<VNode> = rows
        .iter()
        .map(|row| {
            let cells: Vec<VNode> = row
                .iter()
                .map(|v| rsx! { td { class: cell.name(), "{v}" } })
                .collect();
            rsx! { tr { {cells} } }
        })
        .collect();
    rsx! {
        table { class: table.name(),
            thead { tr { {head} } }
            tbody { {body} }
        }
    }
}

/// A plain or ordered list.
#[component]
pub fn List(items: Vec<String>, ordered: bool) -> VNode {
    let list = css! { margin: 0; padding-inline-start: 20px; };
    let nodes: Vec<VNode> = items.iter().map(|i| rsx! { li { "{i}" } }).collect();
    if *ordered {
        rsx! { ol { class: list.name(), {nodes} } }
    } else {
        rsx! { ul { class: list.name(), {nodes} } }
    }
}

/// A collapsible content section.
#[derive(Clone, Debug)]
pub struct AccordionItem {
    pub title: String,
    pub content: VNode,
}

/// An accordion: one item expanded at a time (controlled by `open`).
#[component]
pub fn Accordion(items: Vec<AccordionItem>, open: usize) -> VNode {
    let box_ = css! { border: 1px solid var(--muted); border-radius: var(--radius); };
    let header = css! {
        all: unset;
        box-sizing: border-box;
        display: block;
        width: 100%;
        cursor: pointer;
        padding: 10px 12px;
        font-weight: 600;
    };
    let body = css! { padding: 0 12px 12px 12px; color: var(--muted); };
    let rows: Vec<VNode> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let expanded = i == *open;
            let content = if expanded {
                rsx! { div { class: body.name(), {item.content.clone()} } }
            } else {
                VNode::Empty
            };
            rsx! {
                div { class: box_.name(),
                    button { class: header.name(), aria-expanded: expanded, "{item.title}" }
                    {content}
                }
            }
        })
        .collect();
    rsx! { div { {rows} } }
}

// ---------------------------------------------------------------------------
// Forms
// ---------------------------------------------------------------------------

/// A text input. `input_type` is the DOM `type` (`text`, `email`, ...).
#[component]
pub fn Input(
    value: String,
    placeholder: String,
    input_type: String,
    on_input: Option<EventHandler>,
) -> VNode {
    let id = use_id();
    let input = css! {
        box-sizing: border-box;
        padding: 8px 10px;
        border: 1px solid var(--muted);
        border-radius: var(--radius);
        background: var(--bg);
        color: var(--fg);
        font: inherit;
        width: 100%;
    };
    let attr = format!("ferrum-input-{}", id);
    match on_input {
        Some(handler) => {
            let h2 = handler.clone();
            rsx! {
                input {
                    id: attr,
                    class: input.name(),
                    type: input_type,
                    placeholder: placeholder,
                    value: value,
                    oninput: move |e: &ferrum_core::FerrumEvent| h2(e),
                }
            }
        }
        None => rsx! {
            input {
                id: attr,
                class: input.name(),
                type: input_type,
                placeholder: placeholder,
                value: value,
            }
        },
    }
}

/// A multi-line text input.
#[component]
pub fn Textarea(
    value: String,
    placeholder: String,
    rows: usize,
    on_input: Option<EventHandler>,
) -> VNode {
    let rows = *rows;
    let area = css! {
        box-sizing: border-box;
        padding: 8px 10px;
        border: 1px solid var(--muted);
        border-radius: var(--radius);
        background: var(--bg);
        color: var(--fg);
        font: inherit;
        width: 100%;
        resize: vertical;
    };
    match on_input {
        Some(handler) => {
            let h2 = handler.clone();
            rsx! {
                textarea {
                    class: area.name(),
                    placeholder: placeholder,
                    rows: rows,
                    oninput: move |e: &ferrum_core::FerrumEvent| h2(e),
                    "{value}"
                }
            }
        }
        None => rsx! {
            textarea { class: area.name(), placeholder: placeholder, rows: rows, "{value}" }
        },
    }
}

/// A select box.
#[component]
pub fn Select(options: Vec<String>, value: String, on_change: Option<EventHandler>) -> VNode {
    let select = css! {
        box-sizing: border-box;
        padding: 8px 10px;
        border: 1px solid var(--muted);
        border-radius: var(--radius);
        background: var(--bg);
        color: var(--fg);
        font: inherit;
    };
    let opts: Vec<VNode> = options
        .iter()
        .map(|o| {
            let selected = *o == *value;
            rsx! { option { value: o.clone(), selected: selected, "{o}" } }
        })
        .collect();
    match on_change {
        Some(handler) => {
            let h2 = handler.clone();
            rsx! {
                select { class: select.name(), onchange: move |e: &ferrum_core::FerrumEvent| h2(e), {opts} }
            }
        }
        None => rsx! { select { class: select.name(), {opts} } },
    }
}

/// A checkbox with a label.
#[component]
pub fn Checkbox(checked: bool, label: String, on_change: Option<EventHandler>) -> VNode {
    let checked = *checked;
    let row = css! { display: inline-flex; align-items: center; gap: 8px; cursor: pointer; };
    let handler = on_change.clone();
    let input = match handler {
        Some(h) => rsx! {
            input {
                type: "checkbox",
                checked: checked,
                onchange: move |e: &ferrum_core::FerrumEvent| h(e),
            }
        },
        None => rsx! { input { type: "checkbox", checked: checked } },
    };
    rsx! { label { class: row.name(), {input} span { "{label}" } } }
}

/// A radio button.
#[component]
pub fn Radio(
    name: String,
    value: String,
    checked: bool,
    label: String,
    on_change: Option<EventHandler>,
) -> VNode {
    let checked = *checked;
    let row = css! { display: inline-flex; align-items: center; gap: 8px; cursor: pointer; };
    let handler = on_change.clone();
    let input = match handler {
        Some(h) => rsx! {
            input {
                type: "radio",
                name: name,
                value: value,
                checked: checked,
                onchange: move |e: &ferrum_core::FerrumEvent| h(e),
            }
        },
        None => rsx! { input { type: "radio", name: name, value: value, checked: checked } },
    };
    rsx! { label { class: row.name(), {input} span { "{label}" } } }
}

/// A switch (a checkbox with a track).
#[component]
pub fn Switch(checked: bool, label: String, on_change: Option<EventHandler>) -> VNode {
    let checked = *checked;
    let row = css! { display: inline-flex; align-items: center; gap: 8px; cursor: pointer; };
    let track = css! {
        width: 40px;
        height: 22px;
        border-radius: 999px;
        background: var(--muted);
        position: relative;
        display: inline-block;
    };
    let knob = css! {
        position: absolute;
        top: 3px;
        inset-inline-start: 3px;
        width: 16px;
        height: 16px;
        border-radius: 50%;
        background: var(--bg);
    };
    let handler = on_change.clone();
    let input = match handler {
        Some(h) => rsx! {
            input {
                type: "checkbox",
                checked: checked,
                onchange: move |e: &ferrum_core::FerrumEvent| h(e),
            }
        },
        None => rsx! { input { type: "checkbox", checked: checked } },
    };
    rsx! {
        label { class: row.name(),
            {input}
            span { class: track.name(), aria-hidden: true, span { class: knob.name() } }
            span { "{label}" }
        }
    }
}

/// A form label. `for_id` is the `for` attribute.
#[component]
pub fn Label(for_id: String, text: String) -> VNode {
    let label = css! { display: block; margin-bottom: 4px; font-weight: 600; };
    rsx! { label { class: label.name(), for: for_id, "{text}" } }
}

/// A label + control + error message.
#[component]
pub fn FormField(label: String, error: String, children: Vec<VNode>) -> VNode {
    let field = css! { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; };
    let err = css! { color: #b91c1c; font-size: 0.85rem; };
    let error_node = if error.is_empty() {
        VNode::Empty
    } else {
        rsx! { span { class: err.name(), role: "alert", "{error}" } }
    };
    rsx! {
        div { class: field.name(),
            Label { for_id: "", text: label }
            <> {children} {error_node} </>
        }
    }
}

/// A range slider.
#[component]
pub fn Slider(min: i32, max: i32, value: i32, on_input: Option<EventHandler>) -> VNode {
    let (min, max, value) = (*min, *max, *value);
    let slider = css! { width: 100%; accent-color: var(--accent); };
    match on_input {
        Some(handler) => {
            let h2 = handler.clone();
            rsx! {
                input {
                    class: slider.name(),
                    type: "range",
                    min: min,
                    max: max,
                    value: value,
                    oninput: move |e: &ferrum_core::FerrumEvent| h2(e),
                }
            }
        }
        None => {
            rsx! { input { class: slider.name(), type: "range", min: min, max: max, value: value } }
        }
    }
}

// ---------------------------------------------------------------------------
// Overlays
// ---------------------------------------------------------------------------

/// A modal dialog. Render nothing when `open` is false.
#[component]
pub fn Modal(
    open: bool,
    title: String,
    on_close: Option<EventHandler>,
    children: Vec<VNode>,
) -> VNode {
    if !*open {
        return VNode::Empty;
    }
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
        border-radius: var(--radius);
        padding: 20px;
        min-width: 320px;
        max-width: 90vw;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
    };
    let head =
        css! { display: flex; justify-content: space-between; align-items: center; gap: 16px; };
    let close = css! { all: unset; cursor: pointer; font-size: 1.2rem; line-height: 1; };
    let close_node = match on_close {
        Some(handler) => {
            let h2 = handler.clone();
            rsx! { button { class: close.name(), aria-label: "Close", onclick: move |e: &ferrum_core::FerrumEvent| h2(e), "×" } }
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

/// A tooltip wrapper (CSS-only hover/focus reveal).
#[component]
pub fn Tooltip(text: String, children: Vec<VNode>) -> VNode {
    let wrap = css! { position: relative; display: inline-block; };
    let bubble = css! {
        position: absolute;
        bottom: 100%;
        inset-inline-start: 50%;
        transform: translateX(-50%);
        background: var(--fg);
        color: var(--bg);
        padding: 4px 8px;
        border-radius: 4px;
        font-size: 0.8rem;
        white-space: nowrap;
        opacity: 0;
        pointer-events: none;
    };
    rsx! {
        span { class: wrap.name(),
            {children}
            span { class: bubble.name(), role: "tooltip", "{text}" }
        }
    }
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------

/// One tab: a label and its content.
#[derive(Clone, Debug)]
pub struct TabItem {
    pub label: String,
    pub content: VNode,
}

/// Tabs. `active` is the selected index.
#[component]
pub fn Tabs(tabs: Vec<TabItem>, active: usize, on_change: Option<EventHandler>) -> VNode {
    let bar = css! { display: flex; gap: 4px; border-bottom: 1px solid var(--muted); };
    let tab = css! {
        all: unset;
        box-sizing: border-box;
        cursor: pointer;
        padding: 8px 12px;
        border-bottom: 2px solid transparent;
    };
    let active_tab = css! { border-bottom-color: var(--accent); font-weight: 600; };
    let panel = css! { padding: 12px 0; };
    let buttons: Vec<VNode> = tabs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let class = if i == *active {
                format!("{} {}", tab.name(), active_tab.name())
            } else {
                tab.name().to_string()
            };
            let handler = on_change.clone();
            match handler {
                Some(h) => {
                    let idx = i;
                    rsx! {
                        button {
                            class: class,
                            role: "tab",
                            aria-selected: i == *active,
                            onclick: move |e: &ferrum_core::FerrumEvent| {
                                // The handler receives the index via a custom payload.
                                let ev = ferrum_core::FerrumEvent::new("tab", Box::new(idx));
                                h(&ev);
                                let _ = e;
                            },
                            "{t.label}"
                        }
                    }
                }
                None => rsx! { button { class: class, role: "tab", aria-selected: i == *active, "{t.label}" } },
            }
        })
        .collect();
    let content = tabs
        .get(*active)
        .map(|t| t.content.clone())
        .unwrap_or(VNode::Empty);
    rsx! {
        div {
            div { class: bar.name(), role: "tablist", {buttons} }
            div { class: panel.name(), role: "tabpanel", {content} }
        }
    }
}

/// One menu entry.
#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub on_select: Option<EventHandler>,
}

/// A dropdown menu.
#[component]
pub fn Menu(items: Vec<MenuItem>, open: bool) -> VNode {
    let wrap = css! { position: relative; display: inline-block; };
    let list = css! {
        position: absolute;
        top: 100%;
        inset-inline-start: 0;
        min-width: 160px;
        background: var(--bg);
        border: 1px solid var(--muted);
        border-radius: var(--radius);
        padding: 4px;
        z-index: 10;
    };
    let item = css! {
        all: unset;
        box-sizing: border-box;
        display: block;
        width: 100%;
        cursor: pointer;
        padding: 6px 10px;
        border-radius: 4px;
    };
    let entries: Vec<VNode> = items
        .iter()
        .map(|i| match &i.on_select {
            Some(h) => {
                let h2 = h.clone();
                rsx! { button { class: item.name(), role: "menuitem", onclick: move |e: &ferrum_core::FerrumEvent| h2(e), "{i.label}" } }
            }
            None => rsx! { button { class: item.name(), role: "menuitem", "{i.label}" } },
        })
        .collect();
    let popup = if *open {
        rsx! { div { class: list.name(), role: "menu", {entries} } }
    } else {
        VNode::Empty
    };
    rsx! { div { class: wrap.name(), {popup} } }
}

/// Page navigation.
#[component]
pub fn Pagination(page: usize, pages: usize, on_change: Option<EventHandler>) -> VNode {
    let (page, pages) = (*page, *pages);
    let nav = css! { display: flex; gap: 6px; align-items: center; };
    let btn = css! {
        all: unset;
        cursor: pointer;
        padding: 4px 10px;
        border: 1px solid var(--muted);
        border-radius: var(--radius);
    };
    let disabled = css! { opacity: 0.4; cursor: default; };
    let prev_class = if page <= 1 {
        format!("{} {}", btn.name(), disabled.name())
    } else {
        btn.name().to_string()
    };
    let next_class = if page >= pages {
        format!("{} {}", btn.name(), disabled.name())
    } else {
        btn.name().to_string()
    };
    let prev = match &on_change {
        Some(h) if page > 1 => {
            let h2 = h.clone();
            rsx! {
                button { class: prev_class, onclick: move |e: &ferrum_core::FerrumEvent| {
                    h2(&ferrum_core::FerrumEvent::new("page", Box::new(page - 1)));
                    let _ = e;
                }, "Prev" }
            }
        }
        _ => rsx! { button { class: prev_class, disabled: true, "Prev" } },
    };
    let next = match &on_change {
        Some(h) if page < pages => {
            let h2 = h.clone();
            rsx! {
                button { class: next_class, onclick: move |e: &ferrum_core::FerrumEvent| {
                    h2(&ferrum_core::FerrumEvent::new("page", Box::new(page + 1)));
                    let _ = e;
                }, "Next" }
            }
        }
        _ => rsx! { button { class: next_class, disabled: true, "Next" } },
    };
    rsx! {
        nav { class: nav.name(), aria-label: "Pagination",
            {prev}
            span { "Page {page} / {pages}" }
            {next}
        }
    }
}

/// A breadcrumb trail.
#[component]
pub fn Breadcrumb(items: Vec<String>) -> VNode {
    let nav = css! { display: flex; gap: 6px; font-size: 0.9rem; color: var(--muted); };
    let nodes: Vec<VNode> = items.iter().map(|i| rsx! { span { "{i}" } }).collect();
    rsx! { nav { class: nav.name(), aria-label: "Breadcrumb", {nodes} } }
}

// ---------------------------------------------------------------------------
// Feedback
// ---------------------------------------------------------------------------

/// An inline alert.
#[component]
pub fn Alert(level: String, title: String, children: Vec<VNode>) -> VNode {
    let box_ = css! {
        border: 1px solid var(--muted);
        border-inline-start: 4px solid var(--accent);
        border-radius: var(--radius);
        padding: 10px 12px;
        background: var(--bg);
        color: var(--fg);
    };
    let title_cls = css! { font-weight: 600; margin: 0 0 2px 0; };
    let _ = level; // reserved for severity styling
    rsx! {
        div { class: box_.name(), role: "alert",
            p { class: title_cls.name(), "{title}" }
            {children}
        }
    }
}

/// A transient toast message.
#[component]
pub fn Toast(message: String, level: String) -> VNode {
    let toast = css! {
        display: inline-block;
        padding: 10px 14px;
        border-radius: var(--radius);
        background: var(--fg);
        color: var(--bg);
        box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
    };
    let _ = level;
    rsx! { div { class: toast.name(), role: "status", "{message}" } }
}

/// A progress bar.
#[component]
pub fn Progress(value: i32, max: i32) -> VNode {
    let track = css! {
        width: 100%;
        height: 10px;
        background: var(--muted);
        border-radius: 999px;
        overflow: hidden;
    };
    let fill = css! { height: 100%; background: var(--accent); };
    let max = *max;
    let value = *value;
    let pct = if max <= 0 {
        0
    } else {
        ((value.max(0).min(max) as f64 / max as f64) * 100.0) as i32
    };
    let style = format!("width: {}%;", pct);
    rsx! {
        div { class: track.name(), role: "progressbar", aria-valuenow: value, aria-valuemin: 0, aria-valuemax: max,
            div { class: fill.name(), style: style }
        }
    }
}

/// An indeterminate spinner.
#[component]
pub fn Spinner(label: String) -> VNode {
    let spin = css! {
        display: inline-block;
        width: 18px;
        height: 18px;
        border: 2px solid var(--muted);
        border-top-color: var(--accent);
        border-radius: 50%;
        animation: ferrum-spin 0.8s linear infinite;
    };
    let row = css! { display: inline-flex; align-items: center; gap: 8px; };
    rsx! {
        span { class: row.name(), role: "status",
            span { class: spin.name(), aria-hidden: true }
            span { "{label}" }
        }
    }
}

/// A skeleton placeholder.
#[component]
pub fn Skeleton(lines: usize, width: String) -> VNode {
    let bar = css! {
        height: 12px;
        border-radius: 4px;
        background: var(--muted);
        opacity: 0.5;
        margin-bottom: 8px;
    };
    let style = if width.is_empty() {
        "100%".to_string()
    } else {
        width.clone()
    };
    let rows: Vec<VNode> = (0..*lines)
        .map(|_| rsx! { div { class: bar.name(), style: style.clone() } })
        .collect();
    rsx! { div { aria-hidden: true, {rows} } }
}
