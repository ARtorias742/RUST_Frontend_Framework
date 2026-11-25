//! # ferrum-ui
//!
//! The Ferrum UI component set. These are the **source-distributed** versions
//! of the components `ferrum add` writes into an app; the crate exists so the
//! same source can be tested on web, headless and native.
//!
//! The design rules:
//!
//! - semantic tokens only (`var(--bg)`, `var(--accent)`, ...);
//! - accessibility attributes and keyboard behavior are part of the component;
//! - no hidden state in controlled components;
//! - native and web render the same VNode tree.

use ferrum_core::{component, rsx, EventHandler, VNode};
use ferrum_styling::css;
use ferrum_ui_core::{FocusScope, RovingTabIndex};

/// A button. `variant`: "primary" (default) or "ghost".
#[component(default)]
pub fn Button(
    variant: String,
    disabled: bool,
    on_click: Option<EventHandler>,
    children: Vec<VNode>,
) -> VNode {
    let disabled = *disabled;
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
    let head =
        css! { display: flex; justify-content: space-between; align-items: center; gap: 16px; };
    let close = css! { all: unset; cursor: pointer; font-size: 1.2rem; line-height: 1; };
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
// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

/// One menu entry.
#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub disabled: bool,
    pub on_select: Option<EventHandler>,
}

/// A dropdown menu. `children` is the trigger.
#[component]
pub fn Menu(open: bool, active: usize, items: Vec<MenuItem>, children: Vec<VNode>) -> VNode {
    let open = *open;
    let active = *active;
    let wrap = css! { position: relative; display: inline-block; };
    let list = css! {
        position: absolute;
        top: 100%;
        inset-inline-start: 0;
        min-width: 160px;
        background: var(--bg);
        border: 1px solid var(--border);
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
    let roving = RovingTabIndex::new(active);
    let entries: Vec<VNode> = items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let class = item.name().to_string();
            let tabindex = roving.tab_index(i);
            let disabled = entry.disabled;
            match &entry.on_select {
                Some(handler) if !disabled => {
                    let h = handler.clone();
                    rsx! {
                        button {
                            class: class,
                            role: "menuitem",
                            tabindex: tabindex as i32,
                            onclick: move |e: &ferrum_core::FerrumEvent| h(e),
                            "{entry.label}"
                        }
                    }
                }
                _ => rsx! {
                    button {
                        class: class,
                        role: "menuitem",
                        tabindex: tabindex as i32,
                        aria-disabled: disabled,
                        "{entry.label}"
                    }
                },
            }
        })
        .collect();
    let popup = if open {
        rsx! { div { class: list.name(), role: "menu", {entries} } }
    } else {
        VNode::Empty
    };
    rsx! { div { class: wrap.name(), {children} {popup} } }
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

/// One tab: a label and its panel.
#[derive(Clone)]
pub struct TabItem {
    pub label: String,
    pub content: VNode,
}

/// Tabs with roving tabindex and arrow-key semantics.
#[component]
pub fn Tabs(
    orientation: String,
    active: usize,
    on_change: Option<EventHandler>,
    items: Vec<TabItem>,
) -> VNode {
    let active = *active;
    let vertical = orientation.as_str() == "vertical";
    let bar = css! { display: flex; gap: 4px; border-bottom: 1px solid var(--border); };
    let tab = css! {
        all: unset;
        box-sizing: border-box;
        cursor: pointer;
        padding: 8px 12px;
        border-bottom: 2px solid transparent;
    };
    let active_tab = css! { border-bottom-color: var(--accent); font-weight: 600; };
    let panel = css! { padding: 12px 0; };
    let roving = RovingTabIndex::new(active);
    let buttons: Vec<VNode> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let class = if i == active {
                format!("{} {}", tab.name(), active_tab.name())
            } else {
                tab.name().to_string()
            };
            let tabindex = roving.tab_index(i);
            let handler = on_change.clone();
            let selected = i == active;
            match handler {
                Some(h) => {
                    let idx = i;
                    rsx! {
                        button {
                            class: class,
                            role: "tab",
                            id: format!("ferrum-tab-{idx}"),
                            aria-controls: format!("ferrum-panel-{idx}"),
                            aria-selected: selected,
                            tabindex: tabindex as i32,
                            onclick: move |e: &ferrum_core::FerrumEvent| {
                                h(&ferrum_core::FerrumEvent::new("tab", Box::new(idx)));
                                let _ = e;
                            },
                            "{item.label}"
                        }
                    }
                }
                None => rsx! {
                    button {
                        class: class,
                        role: "tab",
                        id: format!("ferrum-tab-{i}"),
                        aria-controls: format!("ferrum-panel-{i}"),
                        aria-selected: selected,
                        tabindex: tabindex as i32,
                        "{item.label}"
                    }
                },
            }
        })
        .collect();
    let content = items
        .get(active)
        .map(|item| item.content.clone())
        .unwrap_or(VNode::Empty);
    let aria_orientation = if vertical { "vertical" } else { "horizontal" };
    rsx! {
        div {
            div { class: bar.name(), role: "tablist", aria-orientation: aria_orientation, {buttons} }
            div {
                class: panel.name(),
                role: "tabpanel",
                id: format!("ferrum-panel-{active}"),
                aria-labelledby: format!("ferrum-tab-{active}"),
                {content}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Toast
// ---------------------------------------------------------------------------

/// Toast severity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastLevel {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastLevel {
    fn role(self) -> &'static str {
        match self {
            ToastLevel::Error => "alert",
            _ => "status",
        }
    }
}

/// One toast message.
#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub level: ToastLevel,
    pub title: String,
    pub description: String,
}

/// A toast region. The host owns the queue and max-visible policy.
#[component]
pub fn Toaster(toasts: Vec<Toast>, on_dismiss: Option<EventHandler>) -> VNode {
    let region = css! {
        position: fixed;
        inset-inline-end: 16px;
        bottom: 16px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        z-index: 2000;
    };
    let toast = css! {
        min-width: 240px;
        padding: 10px 14px;
        border-radius: var(--radius);
        background: var(--fg);
        color: var(--bg);
    };
    let rows: Vec<VNode> = toasts
        .iter()
        .map(|t| {
            let id = t.id;
            let role = t.level.role();
            let dismiss = on_dismiss.clone();
            let close = match dismiss {
                Some(h) => {
                    let h2 = h.clone();
                    rsx! {
                        button {
                            aria-label: "Dismiss",
                            onclick: move |e: &ferrum_core::FerrumEvent| {
                                h2(&ferrum_core::FerrumEvent::new("dismiss", Box::new(id)));
                                let _ = e;
                            },
                            "×"
                        }
                    }
                }
                None => rsx! { span { aria-hidden: true, "×" } },
            };
            rsx! {
                div { class: toast.name(), role: role,
                    div { "{t.title}" }
                    div { "{t.description}" }
                    {close}
                }
            }
        })
        .collect();
    rsx! { div { class: region.name(), aria-live: "polite", {rows} } }
}
