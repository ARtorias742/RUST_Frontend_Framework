//! Phase 2 web example: controlled form (input + select + submit).
//!
//! Shows controlled inputs (value/checked set as DOM *properties* so typing +
//! re-render works), an `if let` conditional, and a `for` loop building
//! `<option>` elements.

use ferrum_core::rsx;
use ferrum_web::{AppHandle, FerrumEventExt};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

struct FormState {
    name: String,
    lang: String,
    submitted: Option<String>,
}

#[wasm_bindgen(start)]
pub fn start() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let root = document.get_element_by_id("app").expect("#app not found");

    let app = AppHandle::new(root);
    let state = Rc::new(RefCell::new(FormState {
        name: String::new(),
        lang: "Rust".to_string(),
        submitted: None,
    }));

    let s = state.clone();
    let a = app.clone();
    app.render(move || {
        let (name, lang, submitted) = {
            let st = s.borrow();
            (st.name.clone(), st.lang.clone(), st.submitted.clone())
        };

        let s_name = s.clone();
        let a_name = a.clone();
        let s_lang = s.clone();
        let a_lang = a.clone();
        let s_submit = s.clone();
        let a_submit = a.clone();

        let langs = ["Rust", "WASM", "Ferrum"];

        rsx! {
            div { class: "form",
                h1 { "Controlled Form" }
                form {
                    onsubmit: move |e| {
                        e.prevent_default();
                        let msg = {
                            let st = s_submit.borrow();
                            format!("Hello, {}! ({})", st.name, st.lang)
                        };
                        s_submit.borrow_mut().submitted = Some(msg);
                        a_submit.re_render();
                    },

                    label { "Name:" }
                    input {
                        class: "name",
                        value: name,
                        oninput: move |e| {
                            s_name.borrow_mut().name = e.target_value().unwrap_or_default();
                            a_name.re_render();
                        },
                    }

                    label { "Language:" }
                    select {
                        class: "lang",
                        value: lang,
                        oninput: move |e| {
                            s_lang.borrow_mut().lang = e.target_value().unwrap_or_default();
                            a_lang.re_render();
                        },
                        for opt in langs {
                            option { value: opt, "{opt}" }
                        }
                    }

                    button { "Submit" }
                }

                if let Some(msg) = submitted {
                    p { class: "msg", "{msg}" }
                }
            }
        }
    });
    app.forget();
}
