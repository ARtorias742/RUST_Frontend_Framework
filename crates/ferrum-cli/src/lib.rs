//! `ferrum_dev`: the development server library behind `ferrum dev`.
//!
//! - Static file serving (with the HMR client script injected into index.html).
//! - SSE endpoint (`/__ferrum/events`) pushing `css` / `reload` / `error`
//!   events (no WebSocket, no crypto — pure std).
//! - File watcher: `.css` changes push a `css` event (no reload — the client
//!   cache-busts the stylesheet link); any other change triggers a rebuild
//!   (`cargo build --target wasm32-unknown-unknown`) and a `reload` event
//!   with the measured rebuild time.

pub mod server;

pub use server::{start, start_with_build, DevServerHandle};
