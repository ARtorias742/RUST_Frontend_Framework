use std::any::Any;

/// A renderer-agnostic event delivered to user event handlers.
///
/// The concrete platform event (e.g. `web_sys::Event`) is stored in `payload`
/// and can be downcast by the handler. Renderers construct this type; user
/// code only ever sees `&FerrumEvent`.
pub struct FerrumEvent {
    /// DOM-style event type, e.g. `"click"` or `"input"`.
    pub event_type: String,
    /// Platform-specific payload (e.g. `Box<web_sys::Event>` on the web).
    pub payload: Box<dyn Any>,
}

impl FerrumEvent {
    pub fn new(event_type: impl Into<String>, payload: Box<dyn Any>) -> Self {
        FerrumEvent {
            event_type: event_type.into(),
            payload,
        }
    }

    /// Downcast the payload to a concrete platform event type.
    pub fn downcast<T: Any>(&self) -> Option<&T> {
        self.payload.downcast_ref::<T>()
    }
}

/// Event handler stored on elements (`onclick`, `oninput`, ...).
///
/// `Rc` (not `Box`) so that `VNode` trees stay cheaply cloneable — the
/// web renderer clones handlers into per-mount JS closures.
pub type EventHandler = std::rc::Rc<dyn Fn(&FerrumEvent)>;
