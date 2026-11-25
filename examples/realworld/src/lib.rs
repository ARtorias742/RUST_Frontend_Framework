//! A real (small) app: a persistent task board.
//!
//! This is the demo the alpha points at. It uses the public API only:
//! components, hooks, a form control, scoped styling, and `localStorage`
//! for persistence. It is intentionally not a framework test — it is the
//! kind of app the framework has to make pleasant.
//!
//! Serve with `trunk serve examples/realworld`.

use ferrum_components::{Button, Card, Checkbox, EmptyState, Input, ThemeRoot};
use ferrum_core::{component, rsx, use_effect_every, use_state, FerrumEvent, VNode};
use ferrum_styling::Theme;
use ferrum_ui::Dialog;
use ferrum_web::{AppHandle, FerrumEventExt};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

const STORAGE_KEY: &str = "ferrum.realworld.tasks.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Task {
    id: u64,
    text: String,
    done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Filter {
    All,
    Active,
    Done,
}

impl Filter {
    fn matches(self, task: &Task) -> bool {
        match self {
            Filter::All => true,
            Filter::Active => !task.done,
            Filter::Done => task.done,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Active => "Active",
            Filter::Done => "Done",
        }
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn encode(task: &Task) -> String {
    let text = task.text.replace(['\n', '\t'], " ");
    format!("{}\t{}\t{}", task.done as u8, task.id, text)
}

fn decode(line: &str) -> Option<Task> {
    let mut parts = line.splitn(3, '\t');
    let done = parts.next()?;
    let id = parts.next()?.parse().ok()?;
    let text = parts.next()?.to_string();
    Some(Task {
        id,
        text,
        done: done == "1",
    })
}

fn load_tasks() -> Vec<Task> {
    let Some(storage) = local_storage() else {
        return Vec::new();
    };
    let Ok(Some(raw)) = storage.get_item(STORAGE_KEY) else {
        return Vec::new();
    };
    raw.lines().filter_map(decode).collect()
}

fn save_tasks(tasks: &[Task]) {
    if let Some(storage) = local_storage() {
        let body = tasks.iter().map(encode).collect::<Vec<_>>().join("\n");
        let _ = storage.set_item(STORAGE_KEY, &body);
    }
}

#[component]
fn TaskBoard() -> VNode {
    let (tasks, set_tasks) = use_state(load_tasks);
    let (draft, set_draft) = use_state(String::new);
    let (filter, set_filter) = use_state(|| Filter::All);
    let (about_open, set_about_open) = use_state(|| false);

    // Persist after every commit. A production app would debounce or use a
    // storage adapter; the demo keeps it explicit.
    let persisted = tasks.clone();
    use_effect_every(move || {
        save_tasks(&persisted);
        None
    });

    let remaining = tasks.iter().filter(|t| !t.done).count();
    let visible: Vec<Task> = tasks
        .iter()
        .filter(|t| filter.matches(t))
        .cloned()
        .collect();

    let add = {
        let set_tasks = set_tasks.clone();
        let set_draft = set_draft.clone();
        let draft = draft.clone();
        move |_: &FerrumEvent| {
            let text = draft.trim().to_string();
            if !text.is_empty() {
                set_tasks.update(move |mut ts| {
                    let id = ts.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                    ts.push(Task {
                        id,
                        text,
                        done: false,
                    });
                    ts
                });
                set_draft.set(String::new());
            }
        }
    };

    let has_visible = !visible.is_empty();
    let rows: Vec<VNode> = visible
        .into_iter()
        .map(|task| {
            let id = task.id;
            let toggle = {
                let set_tasks = set_tasks.clone();
                move |_: &FerrumEvent| {
                    set_tasks.update(move |mut ts| {
                        if let Some(t) = ts.iter_mut().find(|t| t.id == id) {
                            t.done = !t.done;
                        }
                        ts
                    });
                }
            };
            let remove = {
                let set_tasks = set_tasks.clone();
                move |_: &FerrumEvent| {
                    set_tasks.update(move |mut ts| {
                        ts.retain(|t| t.id != id);
                        ts
                    });
                }
            };
            let label = task.text.clone();
            rsx! {
                li {
                    class: "row",
                    Checkbox { checked: task.done, label: label, on_change: Some(Rc::new(toggle) as ferrum_core::EventHandler) }
                    Button { variant: "ghost", on_click: Some(Rc::new(remove) as ferrum_core::EventHandler), "×" }
                }
            }
        })
        .collect();

    let filters: Vec<VNode> = [Filter::All, Filter::Active, Filter::Done]
        .iter()
        .map(|f| {
            let f = *f;
            let active = f == filter;
            let set_filter = set_filter.clone();
            let variant = if active { "primary" } else { "ghost" };
            rsx! {
                Button {
                    variant: variant,
                    on_click: Some(Rc::new(move |_: &FerrumEvent| set_filter.set(f)) as ferrum_core::EventHandler),
                    "{f.label()}"
                }
            }
        })
        .collect();

    let body = if !has_visible {
        rsx! {
            EmptyState {
                title: "Nothing here",
                description: "Add a task, or change the filter.",
            }
        }
    } else {
        rsx! { ul { class: "list", {rows} } }
    };

    let on_input: ferrum_core::EventHandler = Rc::new(move |e: &FerrumEvent| {
        set_draft.set(e.target_value().unwrap_or_default());
    });

    rsx! {
        ThemeRoot { theme: Theme::light(),
            div {
                h1 { "Ferrum Realworld" }
                p { "A persistent task board. Refresh the page — your tasks survive." }
                Button {
                    variant: "ghost",
                    on_click: Some(Rc::new({
                        let set_about_open = set_about_open.clone();
                        move |_: &FerrumEvent| set_about_open.set(true)
                    }) as ferrum_core::EventHandler),
                    "About"
                }
                Card { title: "Task board",
                    div { class: "add",
                        Input {
                            value: draft.clone(),
                            placeholder: "What needs doing?",
                            input_type: "text",
                            on_input: Some(on_input),
                        }
                        Button { variant: "primary", on_click: Some(Rc::new(add) as ferrum_core::EventHandler), "Add" }
                    }
                    div { class: "filters", {filters} }
                    p { class: "count", "{remaining} left" }
                    {body}
                }
                p { class: "hint", "Built with the public ferrum API only." }
                Dialog {
                    open: about_open,
                    title: "About this demo",
                    on_close: Some(Rc::new(move |_: &FerrumEvent| set_about_open.set(false)) as ferrum_core::EventHandler),
                    p { "A task board built with ferrum, ferrum-components and ferrum-ui." }
                    p { "The dialog uses the same source on web and native." }
                }
            }
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.get_element_by_id("app").unwrap();
    let app = AppHandle::new(root);
    app.render(|| rsx! { TaskBoard {} });
    app.forget();
}
