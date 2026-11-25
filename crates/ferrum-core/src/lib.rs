//! # ferrum-core
//!
//! Core intermediate representation for the Ferrum frontend framework.
//!
//! This crate is renderer-agnostic: it defines the [`VNode`] tree that the
//! [`rsx!`] macro produces, the [`ComponentProps`] trait that `#[component]`
//! functions implement, stable list [`Key`]s, and the [`FerrumEvent`] type
//! that renderers wrap around platform events.
//!
//! A debug renderer ([`render_debug`]) is included so the IR can be inspected
//! without a DOM (Phase 1 gate).

mod debug;
mod event;
pub mod hooks;
mod key;
mod vnode;

pub use debug::render_debug;
pub use event::{EventHandler, FerrumEvent};
pub use hooks::{
    current_re_render, flush_effects, render_isolated, render_with, use_callback, use_context,
    use_custom_rc, use_effect, use_effect_every, use_id, use_memo, use_reducer, use_ref,
    use_state, Context, EffectCleanup, EffectJob, EffectQueue, HookEvent, HookInfo, HookObserver, Instance,
    ReducerDispatch, StateSetter, set_hook_observer,
};
pub use key::Key;
pub use vnode::{AttrValue, ComponentProps, ProviderNode, VComponent, VElement, VNode};

// Re-export the proc macros so users only need to depend on `ferrum-core`.
pub use ferrum_macro::{component, rsx};