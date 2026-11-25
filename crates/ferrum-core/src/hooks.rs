//! Hooks runtime: per-component-instance state, effects, context, and the
//! rules-of-hooks enforcement.
//!
//! Design (React-shaped, single-threaded):
//! - Each component instance owns a `Vec<HookSlot>`; slots are addressed by
//!   call order (the rules of hooks).
//! - The renderer wraps every `ComponentProps::render()` call in
//!   [`render_with`], which installs a thread-local [`HookContext`].
//! - State setters schedule a re-render via the `re_render` callback the
//!   renderer provides (a `Weak` link in `ferrum-web`, so no Rc cycles).
//! - Effects are queued during render and flushed by the host **after the
//!   DOM commit** ([`flush_effects`]); their cleanups run on unmount.
//! - Context values propagate through a thread-local provider stack that the
//!   renderer pushes while traversing `VNode::Provider` subtrees.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::vnode::{ComponentProps, VNode};

// ---------------------------------------------------------------------------
// Slots
// ---------------------------------------------------------------------------

/// A serializable summary of one hook slot, for DevTools inspectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookInfo {
    /// Slot index (call order).
    pub index: usize,
    /// Hook kind: `state`, `effect`, `memo`, `ref`, `reducer`, `id`, `custom`.
    pub kind: &'static str,
}

pub(crate) enum HookSlot {
    Empty,
    State(Box<dyn Any>),
    Effect {
        /// `None` = run after every render; `Some(..)` = run when changed.
        last_deps: Option<Vec<u64>>,
        cleanup: Option<Box<dyn FnOnce()>>,
    },
    Memo {
        deps: Vec<u64>,
        value: Rc<dyn Any>,
    },
    Ref(Box<dyn Any>),
    Reducer(Box<dyn Any>), // ReducerSlot
    Id(u64),
}

struct ReducerSlot {
    state: RefCell<Box<dyn Any>>,
    /// Type-erased `Rc<dyn Fn(A)>` (the dispatch closure).
    dispatch: Box<dyn Any>,
}

impl HookSlot {
    fn kind(&self) -> &'static str {
        match self {
            HookSlot::Empty => "empty",
            HookSlot::State(_) => "state",
            HookSlot::Effect { .. } => "effect",
            HookSlot::Memo { .. } => "memo",
            HookSlot::Ref(_) => "ref",
            HookSlot::Reducer(_) => "reducer",
            HookSlot::Id(_) => "id",
        }
    }

    fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        match self {
            HookSlot::State(b) | HookSlot::Ref(b) | HookSlot::Reducer(b) => b.downcast_ref(),
            HookSlot::Memo { value, .. } => value.downcast_ref(),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Instance + render scope
// ---------------------------------------------------------------------------

thread_local! {
    static CURRENT: RefCell<Option<HookContext>> = const { RefCell::new(None) };
    static PROVIDERS: RefCell<Vec<(u64, Rc<dyn Any>)>> = const { RefCell::new(Vec::new()) };
    static NEXT_INSTANCE_ID: Cell<u64> = const { Cell::new(1) };
    static NEXT_CONTEXT_ID: Cell<u64> = const { Cell::new(1) };
    static HOOK_OBSERVER: RefCell<Option<HookObserver>> = const { RefCell::new(None) };
}

/// A state/reducer mutation, reported to the DevTools timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookEvent {
    /// `set`/`update` on a `useState` slot.
    StateSet {
        instance: u64,
        name: &'static str,
        slot: usize,
        /// Type name of the new value (`i32`, `String`, ...).
        value: &'static str,
    },
    /// `dispatch` on a `useReducer` slot.
    ReducerDispatch {
        instance: u64,
        name: &'static str,
        slot: usize,
    },
}

/// Observer installed by `ferrum-devtools` (thread-local, no-op by default).
pub type HookObserver = Rc<dyn Fn(HookEvent)>;

/// Install (or clear with `None`) the hook observer.
pub fn set_hook_observer(observer: Option<HookObserver>) {
    HOOK_OBSERVER.with(|o| *o.borrow_mut() = observer);
}

fn emit_hook_event(event: HookEvent) {
    HOOK_OBSERVER.with(|o| {
        if let Some(f) = o.borrow().as_ref() {
            f(event);
        }
    });
}

/// A component instance: stable identity + hook slots across re-renders.
pub struct Instance {
    pub id: u64,
    pub name: &'static str,
    hooks: RefCell<Vec<HookSlot>>,
}

impl Instance {
    pub fn new(name: &'static str) -> Rc<Self> {
        let id = NEXT_INSTANCE_ID.with(|c| {
            let v = c.get();
            c.set(v + 1);
            v
        });
        Rc::new(Instance {
            id,
            name,
            hooks: RefCell::new(Vec::new()),
        })
    }

    /// Run all effect cleanups (unmount). Custom slots clean up via Drop.
    pub fn unmount(&self) {
        let mut hooks = self.hooks.borrow_mut();
        for slot in hooks.iter_mut().rev() {
            if let HookSlot::Effect { cleanup, .. } = slot {
                if let Some(c) = cleanup.take() {
                    c();
                }
            }
        }
        hooks.clear();
    }

    /// The hook slots of this instance, in call order (DevTools inspector).
    pub fn hook_infos(&self) -> Vec<HookInfo> {
        self.hooks
            .borrow()
            .iter()
            .enumerate()
            .map(|(index, slot)| HookInfo {
                index,
                kind: slot.kind(),
            })
            .collect()
    }
}

/// Per-render bookkeeping shared by all hooks in one component render.
#[derive(Clone)]
pub struct HookContext {
    instance: Rc<Instance>,
    cursor: Rc<Cell<usize>>,
    re_render: Rc<dyn Fn()>,
    effects: Rc<RefCell<Vec<EffectJob>>>,
}

impl HookContext {
    fn next_slot(&self) -> usize {
        let idx = self.cursor.get();
        self.cursor.set(idx + 1);
        let mut hooks = self.instance.hooks.borrow_mut();
        while hooks.len() <= idx {
            hooks.push(HookSlot::Empty);
        }
        idx
    }

    fn with_slot_mut<R>(&self, idx: usize, f: impl FnOnce(&mut HookSlot) -> R) -> R {
        let mut hooks = self.instance.hooks.borrow_mut();
        f(&mut hooks[idx])
    }
}

/// Effect cleanup returned by an effect closure.
pub type EffectCleanup = Box<dyn FnOnce()>;
/// Effect closure: returns the cleanup to run before the next run / unmount.
pub type EffectFn = Box<dyn FnMut() -> Option<EffectCleanup>>;

pub struct EffectJob {
    instance: Rc<Instance>,
    idx: usize,
    deps: Option<Vec<u64>>,
    f: EffectFn,
}

/// Queue of pending effects for one render pass (created by the host).
pub type EffectQueue = Rc<RefCell<Vec<EffectJob>>>;

/// Render `f` with `instance`'s hook scope installed. Panics if the hook
/// count differs from the previous render (rules of hooks).
pub fn render_with<R>(
    instance: &Rc<Instance>,
    re_render: Rc<dyn Fn()>,
    effects: EffectQueue,
    f: impl FnOnce() -> R,
) -> R {
    let ctx = HookContext {
        instance: instance.clone(),
        cursor: Rc::new(Cell::new(0)),
        re_render,
        effects,
    };
    let prev = CURRENT.with(|c| c.replace(Some(ctx.clone())));
    let out = f();
    CURRENT.with(|c| c.replace(prev));
    let used = ctx.cursor.get();
    let total = instance.hooks.borrow().len();
    if used != total {
        panic!(
            "Ferrum: hook count changed between renders of `{}` (used {used}, stored {total}). \
             Hooks must be called in the same order every render — no hooks inside `if`/`for`/closures.",
            instance.name
        );
    }
    out
}

/// Render props in a throwaway scope (debug renderer, spikes). Effects are
/// discarded; state starts fresh on every call.
pub fn render_isolated(props: &Rc<dyn ComponentProps>) -> VNode {
    let instance = Instance::new("<isolated>");
    let effects: EffectQueue = Rc::new(RefCell::new(Vec::new()));
    render_with(&instance, Rc::new(|| {}), effects, || props.render())
}

fn current_ctx() -> HookContext {
    CURRENT.with(|c| {
        c.borrow()
            .clone()
            .expect("Ferrum: hook called outside of a component render")
    })
}

/// The host's re-render callback for the current component render (used by
/// extension hooks like `ferrum_data::use_query` to register watchers).
pub fn current_re_render() -> Rc<dyn Fn()> {
    current_ctx().re_render
}

/// Run all queued effects (after DOM commit). Effects that set state trigger
/// a nested re-render through the host's `re_render`.
pub fn flush_effects(effects: &EffectQueue) {
    let jobs: Vec<EffectJob> = effects.borrow_mut().drain(..).collect();
    for mut job in jobs {
        let should_run = {
            let hooks = job.instance.hooks.borrow();
            match &hooks[job.idx] {
                HookSlot::Effect { last_deps, .. } => match (&job.deps, last_deps) {
                    (None, _) => true,                  // no deps: every render
                    (Some(d), Some(prev)) => d != prev, // deps changed
                    (Some(_), None) => true,            // first run
                },
                _ => true,
            }
        };
        if should_run {
            // Mark deps consumed *before* running the effect. An effect may
            // synchronously set state (e.g. a lazy chunk resolving), which
            // re-enters the render loop; the nested render must see these
            // deps as already handled instead of scheduling the effect again.
            {
                let mut hooks = job.instance.hooks.borrow_mut();
                if let HookSlot::Effect { last_deps, .. } = &mut hooks[job.idx] {
                    *last_deps = job.deps.clone();
                }
            }
            // React semantics: the previous cleanup runs before the next
            // effect (and on unmount, via Instance::unmount).
            let old_cleanup = {
                let mut hooks = job.instance.hooks.borrow_mut();
                match &mut hooks[job.idx] {
                    HookSlot::Effect { cleanup, .. } => cleanup.take(),
                    _ => None,
                }
            };
            if let Some(c) = old_cleanup {
                c();
            }
            let cleanup = (job.f)();
            let mut hooks = job.instance.hooks.borrow_mut();
            if let HookSlot::Effect { cleanup: c, .. } = &mut hooks[job.idx] {
                *c = cleanup;
            }
        }
    }
}

fn hash_one<T: Hash>(v: &T) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

// ---------------------------------------------------------------------------
// Public hooks
// ---------------------------------------------------------------------------

/// The setter returned by [`use_state`].
#[derive(Clone)]
pub struct StateSetter<T: 'static> {
    ctx: HookContext,
    idx: usize,
    _t: std::marker::PhantomData<fn() -> T>,
}

impl<T: Clone + 'static> StateSetter<T> {
    pub fn set(&self, value: T) {
        let type_name = std::any::type_name_of_val(&value);
        self.ctx.with_slot_mut(self.idx, |slot| {
            *slot = HookSlot::State(Box::new(value));
        });
        emit_hook_event(HookEvent::StateSet {
            instance: self.ctx.instance.id,
            name: self.ctx.instance.name,
            slot: self.idx,
            value: type_name,
        });
        (self.ctx.re_render)();
    }

    pub fn update(&self, f: impl FnOnce(T) -> T) {
        let next = self.ctx.with_slot_mut(self.idx, |slot| {
            let prev = slot
                .downcast_ref::<T>()
                .expect("use_state type changed")
                .clone();
            f(prev)
        });
        let type_name = std::any::type_name_of_val(&next);
        self.ctx.with_slot_mut(self.idx, |slot| {
            *slot = HookSlot::State(Box::new(next));
        });
        emit_hook_event(HookEvent::StateSet {
            instance: self.ctx.instance.id,
            name: self.ctx.instance.name,
            slot: self.idx,
            value: type_name,
        });
        (self.ctx.re_render)();
    }
}

/// `useState`: returns `(value, setter)`. `T: Clone`.
pub fn use_state<T: Clone + 'static>(init: impl FnOnce() -> T) -> (T, StateSetter<T>) {
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let missing = ctx.with_slot_mut(idx, |slot| matches!(slot, HookSlot::Empty));
    if missing {
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::State(Box::new(init()));
        });
    }
    let value = ctx
        .with_slot_mut(idx, |slot| slot.downcast_ref::<T>().cloned())
        .expect("use_state: slot type mismatch");
    (
        value,
        StateSetter {
            ctx,
            idx,
            _t: std::marker::PhantomData,
        },
    )
}

/// `useEffect`: run `f` after commit; return its cleanup. `deps = None`
/// runs after every render; `Some(&[..])` runs when deps change.
pub fn use_effect<D: Hash>(f: impl FnMut() -> Option<EffectCleanup> + 'static, deps: Option<&[D]>) {
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let deps: Option<Vec<u64>> = deps.map(|d| d.iter().map(hash_one).collect());
    let missing = ctx.with_slot_mut(idx, |slot| matches!(slot, HookSlot::Empty));
    if missing {
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::Effect {
                last_deps: None,
                cleanup: None,
            };
        });
    }
    ctx.effects.borrow_mut().push(EffectJob {
        instance: ctx.instance.clone(),
        idx,
        deps,
        f: Box::new(f),
    });
}

/// `useEffect` with no deps: runs after every render.
pub fn use_effect_every(f: impl FnMut() -> Option<EffectCleanup> + 'static) {
    use_effect::<u8>(f, None::<&[u8]>)
}

/// `useMemo`: memoize `f()` until `deps` change. Returns a stable `Rc`.
pub fn use_memo<T: 'static, D: Hash>(deps: &[D], f: impl FnOnce() -> T) -> Rc<T> {
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let deps: Vec<u64> = deps.iter().map(hash_one).collect();
    let reuse = ctx.with_slot_mut(idx, |slot| match slot {
        HookSlot::Memo { deps: d, .. } if d == &deps => true,
        HookSlot::Empty => false,
        _ => false,
    });
    if !reuse {
        let value: Rc<T> = Rc::new(f());
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::Memo { deps, value };
        });
    }
    ctx.with_slot_mut(idx, |slot| match slot {
        HookSlot::Memo { value, .. } => value.clone().downcast::<T>().unwrap(),
        _ => unreachable!(),
    })
}

/// `useCallback`: memoize a callback until `deps` change. Returns a stable
/// `Rc` (call it like `(cb)(args)`).
pub fn use_callback<T: 'static, D: Hash>(deps: &[D], f: T) -> Rc<T> {
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let deps: Vec<u64> = deps.iter().map(hash_one).collect();
    let reuse = ctx.with_slot_mut(
        idx,
        |slot| matches!(slot, HookSlot::Memo { deps: d, .. } if d == &deps),
    );
    if !reuse {
        let value: Rc<T> = Rc::new(f);
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::Memo { deps, value };
        });
    }
    ctx.with_slot_mut(idx, |slot| match slot {
        HookSlot::Memo { value, .. } => value.clone().downcast::<T>().unwrap(),
        _ => unreachable!(),
    })
}

/// `useRef`: a stable mutable container.
pub fn use_ref<T: 'static>(init: T) -> Rc<RefCell<T>> {
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let missing = ctx.with_slot_mut(idx, |slot| matches!(slot, HookSlot::Empty));
    if missing {
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::Ref(Box::new(Rc::new(RefCell::new(init))));
        });
    }
    ctx.with_slot_mut(idx, |slot| {
        slot.downcast_ref::<Rc<RefCell<T>>>()
            .cloned()
            .expect("use_ref: slot type mismatch")
    })
}

/// The dispatch returned by [`use_reducer`].
#[derive(Clone)]
pub struct ReducerDispatch<A: 'static> {
    dispatch: Rc<dyn Fn(A)>,
    _a: std::marker::PhantomData<fn() -> A>,
}

impl<A: 'static> ReducerDispatch<A> {
    pub fn dispatch(&self, action: A) {
        (self.dispatch)(action)
    }
}

/// `useReducer`: returns `(state, dispatch)`.
pub fn use_reducer<S, A>(reducer: impl Fn(&S, A) -> S + 'static, init: S) -> (S, ReducerDispatch<A>)
where
    S: Clone + 'static,
    A: 'static,
{
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let missing = ctx.with_slot_mut(idx, |slot| matches!(slot, HookSlot::Empty));
    if missing {
        type ReducerFn<S, A> = Rc<dyn Fn(&S, A) -> S>;
        let reducer: ReducerFn<S, A> = Rc::new(reducer);
        let dispatch: Rc<dyn Fn(A)> = {
            let ctx = ctx.clone();
            Rc::new(move |action: A| {
                let next = ctx.with_slot_mut(idx, |slot| {
                    let rs = slot
                        .downcast_ref::<ReducerSlot>()
                        .expect("use_reducer: slot type mismatch");
                    let prev = rs.state.borrow().downcast_ref::<S>().unwrap().clone();
                    (reducer)(&prev, action)
                });
                ctx.with_slot_mut(idx, |slot| {
                    let rs = slot
                        .downcast_ref::<ReducerSlot>()
                        .expect("use_reducer: slot type mismatch");
                    *rs.state.borrow_mut() = Box::new(next);
                });
                emit_hook_event(HookEvent::ReducerDispatch {
                    instance: ctx.instance.id,
                    name: ctx.instance.name,
                    slot: idx,
                });
                (ctx.re_render)();
            })
        };
        let dispatch_erased: Box<dyn Any> = Box::new(dispatch);
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::Reducer(Box::new(ReducerSlot {
                state: RefCell::new(Box::new(init)),
                dispatch: dispatch_erased,
            }));
        });
    }
    let state = ctx
        .with_slot_mut(idx, |slot| {
            slot.downcast_ref::<ReducerSlot>()
                .unwrap()
                .state
                .borrow()
                .downcast_ref::<S>()
                .cloned()
        })
        .expect("use_reducer: slot type mismatch");
    let dispatch = ctx.with_slot_mut(idx, |slot| {
        slot.downcast_ref::<ReducerSlot>()
            .unwrap()
            .dispatch
            .downcast_ref::<Rc<dyn Fn(A)>>()
            .cloned()
            .unwrap()
    });
    (
        state,
        ReducerDispatch {
            dispatch,
            _a: std::marker::PhantomData,
        },
    )
}

/// `useId`: a stable id unique to this hook slot.
pub fn use_id() -> u64 {
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let missing = ctx.with_slot_mut(idx, |slot| matches!(slot, HookSlot::Empty));
    if missing {
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::Id(ctx.instance.id * 1024 + idx as u64);
        });
    }
    ctx.with_slot_mut(idx, |slot| match slot {
        HookSlot::Id(v) => *v,
        _ => unreachable!(),
    })
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// A typed context (React Context). Create with [`Context::new`], provide
/// with [`Context::provider`], read with [`use_context`].
#[derive(Clone)]
pub struct Context<T: 'static> {
    id: u64,
    _t: std::marker::PhantomData<fn() -> T>,
}

impl<T: 'static> Default for Context<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: 'static> Context<T> {
    pub fn new() -> Self {
        let id = NEXT_CONTEXT_ID.with(|c| {
            let v = c.get();
            c.set(v + 1);
            v
        });
        Context {
            id,
            _t: std::marker::PhantomData,
        }
    }

    /// Wrap `children` in a provider for this context.
    pub fn provider(&self, value: T, children: Vec<VNode>) -> VNode {
        VNode::Provider(crate::vnode::ProviderNode {
            ctx_id: self.id,
            value: Rc::new(value),
            children,
        })
    }
}

/// `useContext`: read the nearest provider value. `T: Clone`.
pub fn use_context<T: Clone + 'static>(ctx: &Context<T>) -> T {
    let _idx = current_ctx().next_slot(); // consume a slot for order stability
    PROVIDERS.with(|p| {
        let stack = p.borrow();
        for (id, value) in stack.iter().rev() {
            if *id == ctx.id {
                return value
                    .downcast_ref::<T>()
                    .expect("use_context: provider value type mismatch")
                    .clone();
            }
        }
        panic!("use_context: no matching provider in scope")
    })
}

/// Push a provider value for the duration of `f` (called by renderers while
/// traversing `VNode::Provider` subtrees).
pub fn push_provider(ctx_id: u64, value: Rc<dyn Any>) {
    PROVIDERS.with(|p| p.borrow_mut().push((ctx_id, value)));
}

pub fn pop_provider() {
    PROVIDERS.with(|p| {
        p.borrow_mut().pop();
    });
}

// ---------------------------------------------------------------------------
// Custom slots (framework extensions, e.g. ferrum-data's use_query)
// ---------------------------------------------------------------------------

/// `use_custom_rc`: get (or initialize) a custom slot holding
/// `Rc<RefCell<T>>` (stable handle across renders). The slot is dropped when
/// the component unmounts, so `T`'s `Drop` can unregister resources.
pub fn use_custom_rc<T: 'static>(init: impl FnOnce() -> T) -> Rc<RefCell<T>> {
    let ctx = current_ctx();
    let idx = ctx.next_slot();
    let missing = ctx.with_slot_mut(idx, |slot| matches!(slot, HookSlot::Empty));
    if missing {
        ctx.with_slot_mut(idx, |slot| {
            *slot = HookSlot::Ref(Box::new(Rc::new(RefCell::new(init()))));
        });
    }
    ctx.with_slot_mut(idx, |slot| {
        slot.downcast_ref::<Rc<RefCell<T>>>()
            .cloned()
            .expect("use_custom_rc: slot type mismatch")
    })
}
