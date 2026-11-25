//! # ferrum-conformance
//!
//! A WPT-like behavioural suite for the React subset Ferrum implements.
//!
//! The checks live in `tests/conformance.rs` and are run with
//! `cargo test -p ferrum-conformance`. The suite asserts the roadmap's
//! **≥ 95% compatibility gate** and fails on any individual regression.
//!
//! This crate intentionally has no runtime code: it is a test-only gate.

/// The compatibility gate: the suite must pass at least this fraction.
pub const PASS_RATIO_GATE: f64 = 0.95;
