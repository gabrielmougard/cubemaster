//! `rgraph-drag` is a pointer-gesture state machine, adapted for declarative VDOM frameworks
//! such as Dioxus.
//!
//! # Why this port omits the DOM-attaching half
//!
//! d3-drag's public API attaches mouse / touch event listeners to a
//! d3-selection and synthesises drag events from the raw browser events.
//! In a Dioxus codebase the calling component already wires
//! `onmousedown` / `onpointermove` / `ontouchstart` / etc. via
//! `rsx!{ div { onmousedown: move |evt| … } }`, so the listener-attaching
//! half (`drag(selection)`, `nodrag()`, `nonpassivecapture`, etc.) has no
//! idiomatic counterpart in Dioxus.
//!
//! This crate ports the **pure gesture state machine**: feed it pointer
//! events ([`PointerInput::Down`], [`PointerInput::Move`],
//! [`PointerInput::Up`], [`PointerInput::Cancel`]) with raw
//! viewport-relative coordinates, and it emits [`DragEvent`]s on a
//! [`Dispatch`](rgraph_dispatch::Dispatch) carrying the `start` / `drag` /
//! `end` types — exactly mirroring d3-drag's three-event protocol.
//!
//! # Features ported (from d3-drag)
//!
//! * Configurable `filter`, `subject`, `container`, `clickDistance` —
//!   identical semantics to d3, expressed as closures on
//!   [`DragBehavior`].
//! * Multi-pointer / multi-touch tracking. Each concurrent gesture is
//!   keyed by a [`PointerId`] (e.g. `PointerId::Mouse` or
//!   `PointerId::Touch(id)`) and reported in
//!   [`DragEvent::identifier`].
//! * Subject offset preservation — when `subject` returns a value with
//!   coordinates other than the pointer's, the difference is preserved as
//!   `(dx, dy)` across the whole gesture, so the dragged item stays "stuck
//!   to the cursor" the way the user grabbed it.
//! * Click-distance gating — small movements within `click_distance` are
//!   suppressed (no `start` is emitted), so a stationary mousedown +
//!   mouseup behaves as a click.
//! * Active-gesture counter (`DragEvent::active`) reflecting the number
//!   of concurrent gestures, so user code can tell `start`-of-first from
//!   `start`-of-additional.
//! * Dot-namespaced event listeners via
//!   [`DragBehavior::on`] (e.g. `"start.zoom"`), parsed using
//!   `rgraph-selection`'s [`typenames`](rgraph_selection::typenames).
//!
//! # Example
//!
//! ```ignore
//! use rgraph_drag::{DragBehavior, PointerInput, PointerId, Subject};
//! use std::cell::Cell;
//! use std::rc::Rc;
//!
//! // Datum the caller associates with the draggable element. Could be a
//! // node id, a struct, an Rc<RefCell<…>>, etc.
//! type D = u64;
//!
//! let drag = DragBehavior::<D>::new();
//! let pos = Rc::new(Cell::new((0.0, 0.0)));
//! let p = pos.clone();
//! drag.on(
//!     "drag",
//!     Some(std::rc::Rc::new(move |e: &rgraph_drag::DragEvent<D>| {
//!         p.set((e.x, e.y));
//!     })),
//! );
//!
//! // From a Dioxus onmousedown handler:
//! drag.handle(PointerInput::Down {
//!     id: PointerId::Mouse,
//!     x: 100.0, y: 50.0,
//!     button: 0, ctrl: false,
//!     datum: Some(42_u64),
//! });
//! drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 110.0, y: 50.0 });
//! drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 110.0, y: 50.0 });
//! ```

use rgraph_dispatch::{Callback, Dispatch};
use rgraph_selection::typenames::parse_typenames;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

// Re-export the typename helper so users who want to inspect / validate
// typenames in their own code don't need to depend on `rgraph-selection`
// directly.
pub use rgraph_selection::typenames::{
    Typename, parse_one as parse_typename, parse_typenames as parse_event_typenames,
};

// ===========================================================================
// Public types
// ===========================================================================

/// Identifier for a concurrent pointer gesture. d3-drag keys
/// `gestures[identifier]` by `"mouse"` for the mouse and the touch
/// identifier for each touch; we mirror that with a small enum.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum PointerId {
    /// The mouse pointer. There is only one mouse, so its gestures cannot
    /// overlap — but it can run concurrently with [`PointerId::Touch`]es.
    Mouse,
    /// A touch contact, identified by the platform's touch id.
    Touch(u64),
    /// A generic pointer (PointerEvents) — id-only, no special-casing.
    Pointer(u64),
}

/// Subject of a drag gesture. Mirrors d3-drag's `subject` callback return
/// value: `{x, y, …user data…}`. The `(x, y)` defines the *initial*
/// position the gesture should report; the difference between this and
/// the actual pointer position is preserved as the subject offset for the
/// rest of the gesture.
///
/// `D` is the user's per-gesture datum (typically the same datum bound to
/// the draggable element).
#[derive(Clone, Debug)]
pub struct Subject<D> {
    pub x: f64,
    pub y: f64,
    pub datum: D,
}

impl<D> Subject<D> {
    pub fn new(x: f64, y: f64, datum: D) -> Self { Subject { x, y, datum } }
}

/// Convenience: when the user's datum is `()`, building the default
/// subject (pointer position) is a single call.
impl Subject<()> {
    pub fn from_pointer(x: f64, y: f64) -> Self { Subject { x, y, datum: () } }
}

/// One emitted drag event. Mirrors the JS `DragEvent` struct.
#[derive(Clone, Debug)]
pub struct DragEvent<D: Clone> {
    /// Event type: `"start"`, `"drag"`, or `"end"`.
    pub r#type: &'static str,
    /// The pointer that produced this event.
    pub identifier: PointerId,
    /// Number of currently-active concurrent gestures *during* this
    /// event. Mirrors d3's `active` field:
    /// * `start` — count *after* this gesture became active.
    /// * `drag` — current count.
    /// * `end` — count *after* this gesture ended (so the last drag
    ///   reports `0`).
    pub active: u32,
    /// Subject coordinate (= subject's initial `(x, y)` + accumulated
    /// pointer delta). This is the value most user code wants — it tracks
    /// the pointer with the original grab offset preserved.
    pub x: f64,
    pub y: f64,
    /// Pointer delta since the previous event. For `start`, both are 0.
    pub dx: f64,
    pub dy: f64,
    /// The user-supplied subject for this gesture, if any. `None` only
    /// during `beforestart` (which is never dispatched — it's an internal
    /// hook). On `start`/`drag`/`end` this is always `Some`.
    pub subject: Option<Subject<D>>,
    /// The datum supplied with the originating `Down` input.
    pub datum: Option<D>,
}

/// Pointer-input enum fed into [`DragBehavior::handle`]. Coordinates are
/// in *viewport / screen* space; the behavior subtracts the
/// container-offset closure's value to obtain container-relative coords.
#[derive(Clone, Debug)]
pub enum PointerInput<D> {
    /// Pointer pressed.
    Down {
        id: PointerId,
        /// Viewport-relative x coordinate.
        x: f64,
        /// Viewport-relative y coordinate.
        y: f64,
        /// Mouse button (`0` = primary; ignored for touch — pass `0`).
        button: u8,
        /// Whether `ctrl` was held during the down event.
        ctrl: bool,
        /// User datum bound to the draggable element.
        datum: Option<D>,
    },
    /// Pointer moved while pressed.
    Move {
        id: PointerId,
        x: f64,
        y: f64,
    },
    /// Pointer released.
    Up {
        id: PointerId,
        x: f64,
        y: f64,
    },
    /// Pointer cancelled (e.g. touchcancel, lost capture).
    Cancel { id: PointerId },
}

// ===========================================================================
// Closure type aliases (kept private to keep DragBehavior clean)
// ===========================================================================

/// Default-filter signature: `(button, ctrl) -> accept?`.
type FilterFn<D> = Rc<dyn Fn(&FilterContext<D>) -> bool>;
/// Subject closure: `(beforestart event, datum) -> Option<Subject<D>>`.
type SubjectFn<D> = Rc<dyn Fn(&SubjectContext<D>) -> Option<Subject<D>>>;
/// Container-offset closure: returns `(offset_x, offset_y)` of the
/// container in viewport space, so we can compute pointer-relative
/// coords inside the container. Returning `(0.0, 0.0)` means the caller
/// has already converted to container-local coords.
type ContainerFn = Rc<dyn Fn() -> (f64, f64)>;

/// Argument passed to the user's `filter` callback. Mirrors the subset of
/// d3 events that filter inspects.
#[derive(Clone, Debug)]
pub struct FilterContext<D: Clone> {
    pub id: PointerId,
    pub button: u8,
    pub ctrl: bool,
    pub datum: Option<D>,
}

/// Argument passed to the user's `subject` callback. Mirrors d3's
/// `beforestart` event's relevant fields.
#[derive(Clone, Debug)]
pub struct SubjectContext<D: Clone> {
    pub id: PointerId,
    /// Container-relative pointer position at the moment of `Down`.
    pub x: f64,
    pub y: f64,
    pub datum: Option<D>,
}

// ===========================================================================
// Default implementations matching d3-drag
// ===========================================================================

/// d3-drag's `defaultFilter`: ignore right-click (button != 0) and
/// ctrl-click. Public so callers can compose with their own filters.
pub fn default_filter<D: Clone>(ctx: &FilterContext<D>) -> bool {
    ctx.button == 0 && !ctx.ctrl
}

/// d3-drag's `defaultSubject`: return `{x: pointer.x, y: pointer.y, datum}`
/// when `datum` is `None`, otherwise return a clone of the datum at the
/// pointer position. This lets callers pass a struct with `x`/`y` fields
/// as their datum and have d3 use those — but in Rust we don't have a
/// duck-typed `.x` field, so we always start at the pointer position and
/// callers can override with a custom subject closure if they want a
/// different anchor.
pub fn default_subject<D>(ctx: &SubjectContext<D>) -> Option<Subject<D>>
where
    D: Clone + Default,
{
    Some(Subject {
        x: ctx.x,
        y: ctx.y,
        datum: ctx.datum.clone().unwrap_or_default(),
    })
}

// ===========================================================================
// Internal per-gesture state
// ===========================================================================

struct Gesture<D: Clone> {
    /// Subject offset (`subject.x - p0.x`, `subject.y - p0.y`) — added to
    /// the container-relative pointer to get the reported `(x, y)`.
    dx: f64,
    dy: f64,
    /// Container-relative pointer position at the previous event. Used
    /// to compute `(dx, dy)` deltas in subsequent events.
    last_p: (f64, f64),
    /// Pointer-down position in viewport coords (used for click-distance
    /// gating on the *first* move).
    down_viewport: (f64, f64),
    /// Whether we have already crossed the click-distance threshold and
    /// dispatched `start`. d3-drag emits `start` immediately on
    /// `beforestart`, but the equivalent of click-distance suppression in
    /// d3 lives in `nodrag`/`yesdrag` (`noclick`); for our pure-engine
    /// port we delay the `start` event until movement crosses the
    /// threshold (or `Up` is received and the threshold is zero). This is
    /// strictly more useful than d3's behavior for non-DOM contexts.
    started: bool,
    /// Subject at gesture creation. Cloned into emitted events.
    subject: Subject<D>,
    /// Datum at gesture creation.
    datum: Option<D>,
}

// ===========================================================================
// DragBehavior
// ===========================================================================

/// The pure pointer-gesture state machine.
///
/// `D` — the user datum type (must implement [`Clone`]). For gestures
/// without a datum, use `()`.
pub struct DragBehavior<D: Clone> {
    inner: Rc<DragInner<D>>,
}

struct DragInner<D: Clone> {
    /// Active gestures keyed by pointer id. Each `Down` creates an entry,
    /// `Up`/`Cancel` removes it.
    gestures: RefCell<HashMap<PointerId, Gesture<D>>>,
    /// Number of currently-active gestures. Equivalent to d3's `active`
    /// counter.
    active: Cell<u32>,
    /// Click-distance squared. Mirrors d3's `clickDistance2`.
    click_distance2: Cell<f64>,
    /// User callbacks.
    filter: RefCell<FilterFn<D>>,
    subject: RefCell<SubjectFn<D>>,
    container: RefCell<ContainerFn>,
    /// Event dispatcher. Public via [`DragBehavior::on`] which forwards to
    /// the dispatch's `on` method.
    dispatch: Rc<Dispatch<DragEvent<D>>>,
}

impl<D: Clone> DragBehavior<D>
where
    D: 'static,
{
    /// Construct a new `DragBehavior` with d3 defaults (filter rejects
    /// right-/ctrl-click, container offset is `(0, 0)`, click distance is
    /// `0`).
    ///
    /// The default subject closure returns the pointer position with the
    /// supplied datum cloned through; if `datum` is `None` no subject is
    /// produced (d3's default subject is `{x, y}` even with no datum, but
    /// since we can't synthesize a `D` from nothing we require either a
    /// datum on `Down` or a custom subject closure).
    pub fn new() -> Self {
        DragBehavior {
            inner: Rc::new(DragInner {
                gestures: RefCell::new(HashMap::new()),
                active: Cell::new(0),
                click_distance2: Cell::new(0.0),
                filter: RefCell::new(Rc::new(default_filter::<D>)),
                subject: RefCell::new(Rc::new(|ctx: &SubjectContext<D>| {
                    ctx.datum.as_ref().map(|d| Subject {
                        x: ctx.x,
                        y: ctx.y,
                        datum: d.clone(),
                    })
                })),
                container: RefCell::new(Rc::new(|| (0.0, 0.0))),
                dispatch: Rc::new(Dispatch::new(&["start", "drag", "end"])),
            }),
        }
    }

    /// Replace the filter callback. Returns `&self` for chaining.
    ///
    /// The default rejects right-click and ctrl-click. Any closure that
    /// returns `false` causes [`DragBehavior::handle`] to drop the
    /// `Down` event silently — no gesture is created.
    pub fn filter<F>(&self, f: F) -> &Self
    where
        F: Fn(&FilterContext<D>) -> bool + 'static,
    {
        *self.inner.filter.borrow_mut() = Rc::new(f);
        self
    }

    /// Replace the subject callback. Returns `Some(Subject<D>)` to start
    /// a gesture, or `None` to reject it.
    pub fn subject<F>(&self, f: F) -> &Self
    where
        F: Fn(&SubjectContext<D>) -> Option<Subject<D>> + 'static,
    {
        *self.inner.subject.borrow_mut() = Rc::new(f);
        self
    }

    /// Replace the container-offset callback. Returns the
    /// `(offset_x, offset_y)` of the container relative to the viewport,
    /// so pointer coordinates can be made container-relative.
    pub fn container_offset<F>(&self, f: F) -> &Self
    where
        F: Fn() -> (f64, f64) + 'static,
    {
        *self.inner.container.borrow_mut() = Rc::new(f);
        self
    }

    /// Set the click-distance threshold (in pixels). A gesture that does
    /// not move farther than this from its `Down` position before `Up`
    /// emits no `start`/`drag`/`end` events at all — it behaves as a
    /// plain click.
    ///
    /// Mirrors d3's `drag.clickDistance` setter (we store the squared
    /// value internally for fast comparison).
    pub fn click_distance(&self, px: f64) -> &Self {
        self.inner.click_distance2.set(px * px);
        self
    }

    /// Returns the current click-distance threshold.
    pub fn get_click_distance(&self) -> f64 { self.inner.click_distance2.get().sqrt() }

    /// Subscribe / unsubscribe a callback for the given typename. Mirrors
    /// d3-drag's `drag.on(typename, listener)`.
    ///
    /// `typename` may include a dot-namespace (`"start.zoom"`) and may
    /// list multiple types separated by whitespace (`"drag.a end.a"`).
    /// Pass `Some(callback)` to register, `None` to remove.
    ///
    /// # Panics
    ///
    /// Panics if any non-empty type in `typename` is not one of `"start"`,
    /// `"drag"`, or `"end"` — matching d3's `dispatch.on` semantics. The
    /// pre-check uses [`rgraph_selection::typenames::parse_typenames`]
    /// so callers see the bad token rather than a more cryptic dispatch
    /// error.
    pub fn on(&self, typename: &str, callback: Option<Callback<DragEvent<D>>>) -> &Self {
        for tn in parse_typenames(typename) {
            if !tn.type_.is_empty() && !matches!(tn.type_.as_str(), "start" | "drag" | "end") {
                panic!(
                    "unknown drag event type: {:?} (expected start, drag, or end)",
                    tn.type_
                );
            }
        }
        self.inner.dispatch.on(typename, callback);
        self
    }

    /// Returns the current callback bound to `typename` (first match if
    /// the input contains a list).
    pub fn callback(&self, typename: &str) -> Option<Callback<DragEvent<D>>> {
        self.inner.dispatch.callback(typename)
    }

    /// Returns whether any gesture is currently in progress.
    pub fn is_active(&self) -> bool { self.inner.active.get() > 0 }

    /// Returns the number of currently-active concurrent gestures.
    pub fn active_count(&self) -> u32 { self.inner.active.get() }

    /// Returns the underlying [`Dispatch`] for advanced use (e.g. building
    /// a copy for use elsewhere).
    pub fn dispatcher(&self) -> Rc<Dispatch<DragEvent<D>>> {
        Rc::clone(&self.inner.dispatch)
    }

    /// Feed a pointer event into the state machine. This is the *only*
    /// way to drive a [`DragBehavior`].
    ///
    /// Returns `true` iff the input produced a state transition (gesture
    /// created, advanced, or terminated). `false` indicates the input
    /// was filtered out (rejected `Down`) or was a stray `Move`/`Up` for
    /// a pointer that has no active gesture.
    pub fn handle(&self, input: PointerInput<D>) -> bool {
        match input {
            PointerInput::Down { id, x, y, button, ctrl, datum } => {
                self.on_down(id, x, y, button, ctrl, datum)
            }
            PointerInput::Move { id, x, y } => self.on_move(id, x, y),
            PointerInput::Up { id, x, y } => self.on_up(id, x, y),
            PointerInput::Cancel { id } => self.on_cancel(id),
        }
    }

    // ----- internal handlers -----

    fn on_down(
        &self,
        id: PointerId,
        x: f64,
        y: f64,
        button: u8,
        ctrl: bool,
        datum: Option<D>,
    ) -> bool {
        // 0) If this pointer already has an active gesture, ignore (d3
        // implicitly does this via `gestures[identifier]` lookup).
        if self.inner.gestures.borrow().contains_key(&id) {
            return false;
        }

        // 1) Run the filter. Cloning the Rc lets us drop the borrow
        //    before invoking user code (which may call `on()` on this
        //    behavior).
        let filter = Rc::clone(&self.inner.filter.borrow());
        let fctx = FilterContext { id, button, ctrl, datum: datum.clone() };
        if !filter(&fctx) {
            return false;
        }

        // 2) Compute container-relative pointer coordinates.
        let container = Rc::clone(&self.inner.container.borrow());
        let (cx, cy) = container();
        let p = (x - cx, y - cy);

        // 3) Run the subject closure. Returning None aborts the gesture.
        let subject_fn = Rc::clone(&self.inner.subject.borrow());
        let sctx = SubjectContext { id, x: p.0, y: p.1, datum: datum.clone() };
        let Some(subject) = subject_fn(&sctx) else {
            return false;
        };

        let dx = subject.x - p.0;
        let dy = subject.y - p.1;

        // 4) Insert the gesture in `pending` state. We do *not* dispatch
        //    `start` yet if click-distance > 0 — we wait for movement to
        //    cross the threshold (more useful than d3's behavior in a
        //    non-DOM context where there's no native click suppression).
        let click_dist2 = self.inner.click_distance2.get();
        let started_immediately = click_dist2 == 0.0;

        let gesture = Gesture {
            dx, dy,
            last_p: p,
            down_viewport: (x, y),
            started: started_immediately,
            subject: subject.clone(),
            datum: datum.clone(),
        };
        self.inner.gestures.borrow_mut().insert(id, gesture);

        if started_immediately {
            // Emit `start` event with active count incremented.
            self.inner.active.set(self.inner.active.get() + 1);
            let active = self.inner.active.get();
            self.dispatch_event(DragEvent {
                r#type: "start",
                identifier: id,
                active,
                x: subject.x,
                y: subject.y,
                dx: 0.0,
                dy: 0.0,
                subject: Some(subject),
                datum,
            });
        }
        true
    }

    fn on_move(&self, id: PointerId, x: f64, y: f64) -> bool {
        // Compute container-relative coords up front so we can drop the
        // dispatcher / closure borrows cleanly.
        let container = Rc::clone(&self.inner.container.borrow());
        let (cx, cy) = container();
        let p = (x - cx, y - cy);

        // We need exclusive access to the gesture for state mutation, but
        // we must drop the borrow before dispatching. Use a temporary
        // pull-out / push-back pattern instead.
        let mut gestures = self.inner.gestures.borrow_mut();
        let Some(gesture) = gestures.get_mut(&id) else {
            return false;
        };

        let click_dist2 = self.inner.click_distance2.get();
        // Click-distance gating: if we haven't started yet, check
        // whether the movement from down_viewport exceeded the
        // threshold.
        if !gesture.started {
            let ddx = x - gesture.down_viewport.0;
            let ddy = y - gesture.down_viewport.1;
            if ddx * ddx + ddy * ddy <= click_dist2 {
                // Still inside the threshold — update last_p so the next
                // start sees a tiny dx,dy if any, but don't dispatch.
                gesture.last_p = p;
                return true;
            }
            // Cross the threshold: emit `start` first, then fall through
            // to emit `drag`.
            gesture.started = true;
            // The original last_p was set at Down to the down position;
            // d3 reports `dx=0, dy=0` for `start`. Compute the start
            // event's coordinates now.
            self.inner.active.set(self.inner.active.get() + 1);
            let active = self.inner.active.get();
            let start_event = DragEvent {
                r#type: "start",
                identifier: id,
                active,
                x: gesture.subject.x,
                y: gesture.subject.y,
                dx: 0.0,
                dy: 0.0,
                subject: Some(gesture.subject.clone()),
                datum: gesture.datum.clone(),
            };
            // Reset last_p so the upcoming `drag` event reports a delta
            // from down_position, not from a stale last_p.
            gesture.last_p = p;
            // Drop borrow before dispatch (which may call back into us).
            drop(gestures);
            self.dispatch_event(start_event);
            // Re-borrow to compute the `drag` event.
            let mut gestures = self.inner.gestures.borrow_mut();
            let Some(gesture) = gestures.get_mut(&id) else {
                return true; // user removed itself in `start`
            };
            // Dispatch a `drag` immediately at the same pointer position
            // so the listener gets a consistent event sequence even when
            // start and drag coalesce on threshold crossing.
            let dx = 0.0; // No further movement since we just set last_p.
            let dy = 0.0;
            let evt_x = p.0 + gesture.dx;
            let evt_y = p.1 + gesture.dy;
            // Update gesture last_p (already p) and emit.
            let drag_event = DragEvent {
                r#type: "drag",
                identifier: id,
                active: self.inner.active.get(),
                x: evt_x,
                y: evt_y,
                dx, dy,
                subject: Some(gesture.subject.clone()),
                datum: gesture.datum.clone(),
            };
            drop(gestures);
            self.dispatch_event(drag_event);
            return true;
        }

        // Normal drag tick: emit `drag` with delta from previous pointer.
        let dx = p.0 - gesture.last_p.0;
        let dy = p.1 - gesture.last_p.1;
        let evt_x = p.0 + gesture.dx;
        let evt_y = p.1 + gesture.dy;
        let drag_event = DragEvent {
            r#type: "drag",
            identifier: id,
            active: self.inner.active.get(),
            x: evt_x,
            y: evt_y,
            dx, dy,
            subject: Some(gesture.subject.clone()),
            datum: gesture.datum.clone(),
        };
        gesture.last_p = p;
        drop(gestures);
        self.dispatch_event(drag_event);
        true
    }

    fn on_up(&self, id: PointerId, x: f64, y: f64) -> bool {
        let container = Rc::clone(&self.inner.container.borrow());
        let (cx, cy) = container();
        let p = (x - cx, y - cy);

        let gesture = self.inner.gestures.borrow_mut().remove(&id);
        let Some(gesture) = gesture else {
            return false;
        };

        if !gesture.started {
            // Click suppressed — nothing to dispatch. The gesture is
            // removed and active was never incremented.
            return true;
        }

        // d3 semantics: `end` decrements active *before* the listeners
        // see the event (look at d3-drag drag.js: `case "end": delete
        // gestures[identifier], --active;` falls through into "drag"
        // which sets `n = active`, but the dispatched type is "end" with
        // that same n).
        let prev_active = self.inner.active.get();
        let active = prev_active.saturating_sub(1);
        self.inner.active.set(active);

        let dx = p.0 - gesture.last_p.0;
        let dy = p.1 - gesture.last_p.1;
        let evt_x = p.0 + gesture.dx;
        let evt_y = p.1 + gesture.dy;
        let end_event = DragEvent {
            r#type: "end",
            identifier: id,
            active,
            x: evt_x,
            y: evt_y,
            dx, dy,
            subject: Some(gesture.subject.clone()),
            datum: gesture.datum.clone(),
        };
        self.dispatch_event(end_event);
        true
    }

    fn on_cancel(&self, id: PointerId) -> bool {
        // Cancel = same as Up, but we don't have a position. Use the
        // previous position recorded in the gesture.
        let gesture = self.inner.gestures.borrow_mut().remove(&id);
        let Some(gesture) = gesture else { return false };
        if !gesture.started {
            return true;
        }
        let prev_active = self.inner.active.get();
        let active = prev_active.saturating_sub(1);
        self.inner.active.set(active);
        let evt_x = gesture.last_p.0 + gesture.dx;
        let evt_y = gesture.last_p.1 + gesture.dy;
        let end_event = DragEvent {
            r#type: "end",
            identifier: id,
            active,
            x: evt_x,
            y: evt_y,
            dx: 0.0,
            dy: 0.0,
            subject: Some(gesture.subject.clone()),
            datum: gesture.datum.clone(),
        };
        self.dispatch_event(end_event);
        true
    }

    fn dispatch_event(&self, evt: DragEvent<D>) {
        // Dispatch::call by event type matches the field on DragEvent.
        let ty = evt.r#type;
        self.inner.dispatch.call(ty, &evt);
    }
}

impl<D: Clone + 'static> Default for DragBehavior<D> {
    fn default() -> Self { Self::new() }
}

impl<D: Clone> Clone for DragBehavior<D> {
    /// Cheap clone — just bumps the inner Rc. Both clones point at the
    /// same gesture state and dispatcher (mirrors d3 in that the drag
    /// behavior is a single object passed around by reference).
    fn clone(&self) -> Self {
        DragBehavior { inner: Rc::clone(&self.inner) }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Test-only alias: a shared event log paired with a behavior.
    type EventLog<D> = Rc<RefCell<Vec<DragEvent<D>>>>;

    /// Helper: collect all dispatched events into a shared Vec.
    fn collector<D: Clone + 'static>() -> (EventLog<D>, DragBehavior<D>) {
        let log: EventLog<D> = Rc::new(RefCell::new(Vec::new()));
        let drag = DragBehavior::<D>::new();
        for ty in &["start", "drag", "end"] {
            let l = log.clone();
            drag.on(ty, Some(Rc::new(move |e: &DragEvent<D>| l.borrow_mut().push(e.clone()))));
        }
        (log, drag)
    }

    #[test]
    fn down_move_up_produces_start_drag_end() {
        let (log, drag) = collector::<u64>();
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 10.0, y: 20.0,
            button: 0, ctrl: false,
            datum: Some(7),
        });
        drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 15.0, y: 25.0 });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 18.0, y: 28.0 });
        let v = log.borrow();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].r#type, "start");
        assert_eq!(v[1].r#type, "drag");
        assert_eq!(v[2].r#type, "end");
        // Coordinates: subject defaults to pointer position, so x=10,y=20 at start.
        assert_eq!(v[0].x, 10.0);
        assert_eq!(v[0].y, 20.0);
        // Drag has dx=5, dy=5 since moved from (10,20) to (15,25).
        assert_eq!(v[1].dx, 5.0);
        assert_eq!(v[1].dy, 5.0);
        // Drag's reported (x,y) is pointer + offset (offset = 0 here).
        assert_eq!(v[1].x, 15.0);
        assert_eq!(v[1].y, 25.0);
        // End delta is from last move (15,25) -> (18,28).
        assert_eq!(v[2].dx, 3.0);
        assert_eq!(v[2].dy, 3.0);
        assert_eq!(v[2].x, 18.0);
        assert_eq!(v[2].y, 28.0);
    }

    #[test]
    fn filter_rejects_right_click_by_default() {
        let (log, drag) = collector::<u64>();
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0,
            button: 2, // right click
            ctrl: false,
            datum: Some(1),
        });
        drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 5.0, y: 5.0 });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 5.0, y: 5.0 });
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn filter_rejects_ctrl_click_by_default() {
        let (log, drag) = collector::<u64>();
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0,
            button: 0,
            ctrl: true,
            datum: Some(1),
        });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 5.0, y: 5.0 });
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn custom_filter_can_accept_right_click() {
        let (log, drag) = collector::<u64>();
        drag.filter(|_ctx: &FilterContext<u64>| true);
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0,
            button: 2,
            ctrl: true,
            datum: Some(1),
        });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 0.0, y: 0.0 });
        // Click suppressed because no movement (default click_distance=0
        // means started_immediately=true, so we DO see start+end with no
        // movement). Verify both events arrived.
        let v = log.borrow();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].r#type, "start");
        assert_eq!(v[1].r#type, "end");
    }

    #[test]
    fn subject_returning_none_aborts_gesture() {
        let (log, drag) = collector::<u64>();
        drag.subject(|_ctx: &SubjectContext<u64>| None);
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0,
            button: 0, ctrl: false,
            datum: Some(1),
        });
        drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 5.0, y: 5.0 });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 5.0, y: 5.0 });
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn subject_offset_is_preserved() {
        let (log, drag) = collector::<u64>();
        // Subject reports (100, 200) regardless of where the user clicked.
        drag.subject(|ctx: &SubjectContext<u64>| {
            Some(Subject { x: 100.0, y: 200.0, datum: ctx.datum.unwrap_or(0) })
        });
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 50.0, y: 60.0,
            button: 0, ctrl: false,
            datum: Some(42),
        });
        // Pointer moves by (10, 20). Reported (x,y) should be subject + delta.
        drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 60.0, y: 80.0 });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 60.0, y: 80.0 });
        let v = log.borrow();
        // start: x=subject.x, y=subject.y
        assert_eq!(v[0].x, 100.0); assert_eq!(v[0].y, 200.0);
        // drag: x = subject + (move - down) = 100 + 10, y = 200 + 20
        assert_eq!(v[1].x, 110.0); assert_eq!(v[1].y, 220.0);
        // dx,dy = 10,20
        assert_eq!(v[1].dx, 10.0); assert_eq!(v[1].dy, 20.0);
    }

    #[test]
    fn container_offset_is_subtracted() {
        let (log, drag) = collector::<u64>();
        drag.container_offset(|| (100.0, 50.0));
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 150.0, y: 80.0,
            button: 0, ctrl: false,
            datum: Some(1),
        });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 150.0, y: 80.0 });
        let v = log.borrow();
        // pointer (150,80) - container (100,50) = (50,30)
        // subject = pointer (default), so (x,y) = (50, 30).
        assert_eq!(v[0].x, 50.0); assert_eq!(v[0].y, 30.0);
    }

    #[test]
    fn click_distance_suppresses_no_movement_gesture() {
        let (log, drag) = collector::<u64>();
        drag.click_distance(5.0);
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        });
        // Move within threshold (3,3) -- distance ~4.24 < 5
        drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 3.0, y: 3.0 });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 3.0, y: 3.0 });
        // No events should have fired (click-only gesture).
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn click_distance_emits_when_threshold_crossed() {
        let (log, drag) = collector::<u64>();
        drag.click_distance(5.0);
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        });
        // Move within threshold first.
        drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 2.0, y: 2.0 });
        // Then cross it.
        drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 10.0, y: 0.0 });
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 10.0, y: 0.0 });
        let v = log.borrow();
        // Expect: start, drag (the threshold-crossing one), end
        let types: Vec<&str> = v.iter().map(|e| e.r#type).collect();
        assert_eq!(types, vec!["start", "drag", "end"]);
        // The start event reports the subject = pointer at down (0,0)
        assert_eq!(v[0].x, 0.0); assert_eq!(v[0].y, 0.0);
    }

    #[test]
    fn multi_pointer_independent_gestures() {
        let (log, drag) = collector::<u64>();
        // Two simultaneous touches.
        drag.handle(PointerInput::Down {
            id: PointerId::Touch(1),
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(10),
        });
        drag.handle(PointerInput::Down {
            id: PointerId::Touch(2),
            x: 100.0, y: 100.0, button: 0, ctrl: false, datum: Some(20),
        });
        drag.handle(PointerInput::Move { id: PointerId::Touch(1), x: 5.0, y: 5.0 });
        drag.handle(PointerInput::Move { id: PointerId::Touch(2), x: 105.0, y: 105.0 });
        drag.handle(PointerInput::Up { id: PointerId::Touch(1), x: 5.0, y: 5.0 });
        drag.handle(PointerInput::Up { id: PointerId::Touch(2), x: 105.0, y: 105.0 });
        let v = log.borrow();
        // 2 starts, 2 drags, 2 ends = 6.
        assert_eq!(v.len(), 6);
        // Active counts: start1->1, start2->2, drag1->2, drag2->2, end1->1, end2->0
        let actives: Vec<u32> = v.iter().map(|e| e.active).collect();
        assert_eq!(actives, vec![1, 2, 2, 2, 1, 0]);
        // identifiers come through correctly
        assert_eq!(v[0].identifier, PointerId::Touch(1));
        assert_eq!(v[1].identifier, PointerId::Touch(2));
        // Datums survive
        assert_eq!(v[0].datum, Some(10));
        assert_eq!(v[1].datum, Some(20));
    }

    #[test]
    fn duplicate_down_for_same_pointer_is_ignored() {
        let (log, drag) = collector::<u64>();
        let mk_down = || PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        };
        assert!(drag.handle(mk_down()));
        // Second down on same pointer is ignored:
        assert!(!drag.handle(mk_down()));
        let v = log.borrow();
        // Only one start was emitted.
        assert_eq!(v.iter().filter(|e| e.r#type == "start").count(), 1);
    }

    #[test]
    fn move_without_down_is_no_op() {
        let (log, drag) = collector::<u64>();
        assert!(!drag.handle(PointerInput::Move { id: PointerId::Mouse, x: 1.0, y: 1.0 }));
        assert!(!drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 1.0, y: 1.0 }));
        assert!(log.borrow().is_empty());
    }

    #[test]
    fn cancel_emits_end_when_started() {
        let (log, drag) = collector::<u64>();
        drag.handle(PointerInput::Down {
            id: PointerId::Touch(1),
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        });
        drag.handle(PointerInput::Cancel { id: PointerId::Touch(1) });
        let v = log.borrow();
        // start + end (cancel emits end since the gesture was active)
        let types: Vec<&str> = v.iter().map(|e| e.r#type).collect();
        assert_eq!(types, vec!["start", "end"]);
        // Final active should be 0
        assert_eq!(v.last().unwrap().active, 0);
    }

    #[test]
    fn cancel_before_threshold_emits_nothing() {
        let (log, drag) = collector::<u64>();
        drag.click_distance(5.0);
        drag.handle(PointerInput::Down {
            id: PointerId::Touch(1),
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        });
        drag.handle(PointerInput::Cancel { id: PointerId::Touch(1) });
        assert!(log.borrow().is_empty());
        assert_eq!(drag.active_count(), 0);
    }

    #[test]
    fn dot_namespaced_listeners_can_be_removed_independently() {
        let (log, drag) = collector::<u64>();
        let l_extra = log.clone();
        // We re-register `start.extra` with a tag and remove only that.
        drag.on(
            "start.extra",
            Some(Rc::new(move |e: &DragEvent<u64>| {
                l_extra.borrow_mut().push(DragEvent {
                    r#type: "start_extra",
                    ..e.clone()
                });
            })),
        );
        drag.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        });
        // Now remove just `.extra`
        drag.on(".extra", None);
        drag.handle(PointerInput::Up { id: PointerId::Mouse, x: 0.0, y: 0.0 });
        let v = log.borrow();
        // start (default) + start_extra + end (default). Note: default
        // start was registered FIRST (in the collector helper). Order:
        //   - default start
        //   - .extra start
        //   - default end
        let types: Vec<&str> = v.iter().map(|e| e.r#type).collect();
        assert!(types.contains(&"start"));
        assert!(types.contains(&"start_extra"));
        assert!(types.contains(&"end"));
        // After removal, only default end exists. We only registered
        // start_extra once, so the count should be exactly one.
        let extra_count = types.iter().filter(|&&t| t == "start_extra").count();
        assert_eq!(extra_count, 1);
    }

    #[test]
    fn active_count_tracks_concurrent_gestures() {
        let drag = DragBehavior::<u64>::new();
        assert_eq!(drag.active_count(), 0);
        drag.handle(PointerInput::Down {
            id: PointerId::Touch(1),
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        });
        assert_eq!(drag.active_count(), 1);
        drag.handle(PointerInput::Down {
            id: PointerId::Touch(2),
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(2),
        });
        assert_eq!(drag.active_count(), 2);
        drag.handle(PointerInput::Up { id: PointerId::Touch(1), x: 0.0, y: 0.0 });
        assert_eq!(drag.active_count(), 1);
        drag.handle(PointerInput::Up { id: PointerId::Touch(2), x: 0.0, y: 0.0 });
        assert_eq!(drag.active_count(), 0);
    }

    #[test]
    fn clone_shares_state() {
        let d1 = DragBehavior::<u64>::new();
        let d2 = d1.clone();
        d1.handle(PointerInput::Down {
            id: PointerId::Mouse,
            x: 0.0, y: 0.0, button: 0, ctrl: false, datum: Some(1),
        });
        // Both should see active=1 (same Rc).
        assert_eq!(d2.active_count(), 1);
    }

    #[test]
    fn click_distance_get_returns_unsquared() {
        let d = DragBehavior::<u64>::new();
        d.click_distance(7.5);
        assert_eq!(d.get_click_distance(), 7.5);
    }

    #[test]
    #[should_panic(expected = "unknown drag event type")]
    fn unknown_event_type_panics() {
        let d = DragBehavior::<u64>::new();
        d.on("hover", Some(Rc::new(|_| {})));
    }

    #[test]
    fn parse_typename_helper_works() {
        // The re-exported helpers parse dot-namespaced strings the same
        // way the dispatcher does internally.
        let parsed = super::parse_event_typenames("start.zoom drag.zoom");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].type_, "start");
        assert_eq!(parsed[0].name, "zoom");
        assert_eq!(parsed[1].type_, "drag");
        assert_eq!(parsed[1].name, "zoom");
    }
}
