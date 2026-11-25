//! Phase 8 example: the same component as the web example, rendered by the
//! native backend. Prints the scene graph and the accessibility tree.
//!
//! Run: `cargo run -p example-native`

use example_shared::Counter;
use ferrum_core::rsx;
use ferrum_native::NativeApp;

fn main() {
    let app = NativeApp::new();
    app.render(|| {
        rsx! {
            div { class: "app",
                h1 { "Ferrum Native" }
                Counter {}
            }
        }
    });

    println!("scene graph: {}", app.html());
    println!("a11y tree: {:#?}", app.a11y_tree());

    app.fire("click", 0);
    println!("after click: {}", app.text());
}
