//! Optional DevTools observation hook for the reconciler.
//!
//! The reconciler never depends on `ferrum-devtools`; instead it exposes a
//! thread-local [`Observer`] that a DevTools implementation installs with
//! [`set_observer`]. Every component mount / render / unmount is reported,
//! together with the hook-slot summary, so an inspector can reconstruct the
//! live component tree without changing the renderer contract.
//!
//! The observer is a no-op unless installed, so production builds pay only a
//! thread-local `Option` check per component operation.

use ferrum_core::HookInfo;
use std::cell::RefCell;
use std::rc::Rc;

/// Receives component lifecycle events from [`crate::TreeRenderer`].
pub trait Observer {
    /// A component instance was mounted. `parent` is the enclosing component
    /// instance id, if any.
    fn component_mount(&self, id: u64, name: &'static str, parent: Option<u64>, hooks: &[HookInfo]);
    /// A mounted component re-rendered.
    fn component_render(&self, id: u64, name: &'static str, hooks: &[HookInfo]);
    /// A component instance was unmounted (effect cleanups have run).
    fn component_unmount(&self, id: u64, name: &'static str);
}

thread_local! {
    static OBSERVER: RefCell<Option<Rc<dyn Observer>>> = const { RefCell::new(None) };
}

/// Install (or clear with `None`) the reconciler observer.
pub fn set_observer(observer: Option<Rc<dyn Observer>>) {
    OBSERVER.with(|o| *o.borrow_mut() = observer);
}

/// The installed observer, if any.
pub fn observer() -> Option<Rc<dyn Observer>> {
    OBSERVER.with(|o| o.borrow().clone())
}

pub(crate) fn report_mount(id: u64, name: &'static str, parent: Option<u64>, hooks: &[HookInfo]) {
    if let Some(o) = observer() {
        o.component_mount(id, name, parent, hooks);
    }
}

pub(crate) fn report_render(id: u64, name: &'static str, hooks: &[HookInfo]) {
    if let Some(o) = observer() {
        o.component_render(id, name, hooks);
    }
}

pub(crate) fn report_unmount(id: u64, name: &'static str) {
    if let Some(o) = observer() {
        o.component_unmount(id, name);
    }
}