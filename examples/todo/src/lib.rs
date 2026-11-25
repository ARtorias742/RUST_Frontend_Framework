//! Phase 2 web example: todo list with keyed items, add/remove/toggle.
//!
//! Demonstrates the keyed-children reconciler (items keep identity across
//! re-renders) and the pre-hooks render loop: state lives in
//! `Rc<RefCell<State>>` and handlers call `AppHandle::re_render`.

use ferrum_core::rsx;
use ferrum_web::{AppHandle, FerrumEventExt};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

struct Todo {
    id: u64,
    text: String,
    done: bool,
}

struct State {
    items: Vec<Todo>,
    draft: String,
    next_id: u64,
}

#[wasm_bindgen(start)]
pub fn start() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let root = document.get_element_by_id("app").expect("#app not found");

    let app = AppHandle::new(root);
    let state = Rc::new(RefCell::new(State {
        items: Vec::new(),
        draft: String::new(),
        next_id: 1,
    }));

    let s = state.clone();
    let a = app.clone();
    app.render(move || {
        // Snapshot state for this render pass.
        let (items, draft) = {
            let st = s.borrow();
            (
                st.items
                    .iter()
                    .map(|t| (t.id, t.text.clone(), t.done))
                    .collect::<Vec<_>>(),
                st.draft.clone(),
            )
        };

        // Pre-build item nodes (each listener owns its own Rc clones).
        let item_nodes: Vec<ferrum_core::VNode> = items
            .iter()
            .map(|(id, text, done)| {
                let id = *id;
                let text = text.clone();
                let done = *done;
                let s_toggle = s.clone();
                let a_toggle = a.clone();
                let s_del = s.clone();
                let a_del = a.clone();
                rsx! {
                    li {
                        key: id,
                        class: if done { "done" } else { "" },
                        span {
                            class: "check",
                            onclick: move |_| {
                                let mut st = s_toggle.borrow_mut();
                                if let Some(t) = st.items.iter_mut().find(|t| t.id == id) {
                                    t.done = !t.done;
                                }
                                drop(st);
                                a_toggle.re_render();
                            },
                            if done { "[x]" } else { "[ ]" }
                        }
                        span { class: "text", "{text}" }
                        button {
                            class: "del",
                            onclick: move |_| {
                                s_del.borrow_mut().items.retain(|t| t.id != id);
                                a_del.re_render();
                            },
                            "x"
                        }
                    }
                }
            })
            .collect();

        let s_add = s.clone();
        let a_add = a.clone();
        let s_draft = s.clone();
        let a_draft = a.clone();

        rsx! {
            div { class: "todo",
                h1 { "Todo" }
                form {
                    onsubmit: move |e| {
                        e.prevent_default();
                        let mut st = s_add.borrow_mut();
                        let text = st.draft.trim().to_string();
                        if !text.is_empty() {
                            let id = st.next_id;
                            st.next_id += 1;
                            st.items.push(Todo { id, text, done: false });
                        }
                        st.draft = String::new();
                        drop(st);
                        a_add.re_render();
                    },
                    input {
                        class: "draft",
                        placeholder: "What needs doing?",
                        value: draft,
                        oninput: move |e| {
                            s_draft.borrow_mut().draft = e.target_value().unwrap_or_default();
                            a_draft.re_render();
                        },
                    },
                    button { "Add" }
                }
                ul { class: "list", {item_nodes} }
            }
        }
    });
}
