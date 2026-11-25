//! Phase 7 example: server-render a page to HTML (SEO/first paint).
//!
//! Run: `cargo run -p example-ssr`

use ferrum_core::{component, rsx};
use ferrum_ssr::{render_document, render_to_string};

#[component]
fn Home() -> VNode {
    rsx! {
        main {
            h1 { "Ferrum SSR" }
            p { "This HTML is rendered on the server; the client hydrates it." }
            ul {
                li { "Fast first paint" }
                li { "No client JS required to read the page" }
            }
        }
    }
}

fn main() {
    let body = render_to_string(&rsx! { Home {} });
    print!("{}", render_document("Ferrum SSR", &body));
}
