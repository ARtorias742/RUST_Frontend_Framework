//! # ferrum-lazy
//!
//! Code-splitting boundary. [`Lazy`] renders `fallback` until an async
//! `loader` resolves to a `VNode`, then re-renders with the loaded chunk.
//!
//! The loader is a plain future, so it works with:
//! - `wasm_bindgen_futures::spawn_local` + a dynamic `import()` shim (web),
//! - a test executor that polls immediately (native/tests),
//! - any other executor.
//!
//! ```ignore
//! Lazy {
//!     loader: Rc::new(|| Box::pin(async move { load_chunk().await })),
//!     fallback: rsx! { div { "Loading…" } },
//!     executor: Rc::new(wasm_bindgen_futures::spawn_local),
//! }
//! ```

use ferrum_core::{component, use_effect, use_state, VNode};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// The future a chunk loader returns.
pub type LazyFuture = Pin<Box<dyn Future<Output = VNode>>>;
/// The unit-returning task handed to the executor.
pub type LazyTask = Pin<Box<dyn Future<Output = ()>>>;
/// A chunk loader (e.g. dynamic `import()` + wasm instantiation).
pub type LazyLoader = Rc<dyn Fn() -> LazyFuture>;
/// Runs the loader task (e.g. `spawn_local`, or a test queue).
pub type LazyExecutor = Rc<dyn Fn(LazyTask)>;

/// Render `fallback` until `loader` resolves, then render the loaded VNode.
#[component]
pub fn Lazy(loader: LazyLoader, fallback: VNode, executor: LazyExecutor) -> VNode {
    let (state, set_state) = use_state(|| None::<VNode>);
    let loader = loader.clone();
    let executor = executor.clone();
    let set_state = set_state.clone();

    // Run once after mount: start the chunk load. The state setter re-renders
    // this component when the future resolves (no global mutable state).
    use_effect(
        move || {
            let future = loader();
            let set_state = set_state.clone();
            executor(Box::pin(async move {
                let vnode = future.await;
                set_state.set(Some(vnode));
            }));
            None
        },
        Some(&[()]),
    );

    state.unwrap_or_else(|| fallback.clone())
}
