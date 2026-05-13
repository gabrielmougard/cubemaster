//! Transition engine, port of [d3-transition](https://github.com/d3/d3-transition).
//!
//! d3-transition is intrinsically tied to the DOM via d3-selection. This Rust
//! port preserves the **state machine** and **scheduling semantics** of
//! d3-transition while replacing "DOM node" with a generic, user-provided
//! target id `K` (any [`Hash`] + [`Eq`] + [`Clone`] type — typically a `u64`,
//! `String`, or a small newtype). The companion app maps these back to
//! whatever it animates (LEDs, panels, parameters …).
//!
//! # Concepts
//!
//! * [`TransitionEngine`] — owns a [`TimerLoop`](crate::timer::TimerLoop) and
//!   a per-target schedule map.
//! * [`TransitionId`] — opaque, monotonically-increasing handle returned by
//!   [`TransitionEngine::transition`]. Used to query/modify a transition.
//! * **Tween** — a `FnMut(t: f64)` callback invoked on each tick with the
//!   eased progress `t ∈ [0, 1]`. Multiple tweens can be added to the same
//!   transition, each independently named — replacing a tween with the same
//!   name updates it in place.
//! * **Events** — `start`, `end`, `cancel`, `interrupt`, dispatched via
//!   [`rgraph_dispatch::Dispatch`].
//!
//! # State machine
//!
//! Identical to d3-transition:
//!
//! ```text
//! CREATED -> SCHEDULED -> STARTING -> STARTED -> RUNNING -> ENDING -> ENDED
//! ```
//!
//! * Transitions on the same target with the same `name` interrupt one
//!   another — when a newer transition starts, an in-flight one is moved to
//!   `ENDED` and an `interrupt` event is dispatched.
//! * Transitions with smaller ids than a starting transition are cancelled
//!   (they fire `cancel`, not `interrupt`).
//!
//! # Driving
//!
//! Call [`TransitionEngine::tick`] from your render/event loop, providing the
//! current absolute time in milliseconds (or use [`TransitionEngine::tick_now`]
//! to read from the engine's clock). Each tick advances the underlying
//! [`TimerLoop`] which in turn fires schedule callbacks.

use crate::ease::cubic_in_out;
use crate::timer::{Clock, Timer, TimerLoop};
use rgraph_dispatch::Dispatch;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::{Rc, Weak};

// ---------------------------------------------------------------------------
// State enum mirroring d3 constants
// ---------------------------------------------------------------------------

/// Lifecycle states of a single scheduled transition. The numeric
/// representation matches d3 so comparisons (`> SCHEDULED`, `< ENDING`)
/// preserve their meaning.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u8)]
pub enum State {
    Created = 0,
    Scheduled = 1,
    Starting = 2,
    Started = 3,
    Running = 4,
    Ending = 5,
    Ended = 6,
}

// ---------------------------------------------------------------------------
// Public IDs
// ---------------------------------------------------------------------------

/// Opaque, monotonically-increasing transition identifier.
///
/// Returned from [`TransitionEngine::transition`]. Stable across the engine's
/// lifetime and used to reference an in-flight transition for `tween`,
/// `delay`, `duration`, `ease`, `on`, etc.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct TransitionId(pub u64);

// ---------------------------------------------------------------------------
// Tweens
// ---------------------------------------------------------------------------

/// Boxed tween callback. Receives the current eased progress `t ∈ [0, 1]`
/// (or briefly outside it for overshoot easings).
pub type TweenFn = Box<dyn FnMut(f64)>;

struct Tween {
    name: String,
    value: TweenFn,
}

// ---------------------------------------------------------------------------
// Event payload
// ---------------------------------------------------------------------------

/// Argument passed to event listeners. Named `start`, `end`, `cancel`,
/// or `interrupt` are dispatched with this payload.
#[derive(Clone, Debug)]
pub struct EventCtx<K: Clone> {
    /// Target identifier the transition is bound to.
    pub target: K,
    /// Logical name of the transition (empty string for the unnamed default).
    pub name: String,
    /// Transition id.
    pub id: TransitionId,
}

// ---------------------------------------------------------------------------
// Internal schedule
// ---------------------------------------------------------------------------

/// Per-transition schedule, mirrors the JS object held in
/// `node.__transition[id]`.
struct Schedule<K: Hash + Eq + Clone + 'static> {
    target: K,
    name: String,
    id: TransitionId,

    /// Effective scheduling reference time (ms). May be in the past.
    time: f64,
    /// Delay (ms) added on top of `time` before `start` fires.
    delay: Cell<f64>,
    /// Active transition duration (ms).
    duration: Cell<f64>,
    /// Easing function applied to normalized `[0, 1]` progress.
    ease: RefCell<Box<dyn Fn(f64) -> f64>>,

    /// User-attached tweens, in registration order. Replacement preserves
    /// position (matches d3 copy-on-write semantics).
    tweens: RefCell<Vec<Tween>>,

    /// Current lifecycle state.
    state: Cell<State>,

    /// Underlying timer driving this transition's phases.
    timer: RefCell<Option<Timer>>,

    /// Event dispatcher (start / end / cancel / interrupt).
    on: Rc<Dispatch<EventCtx<K>>>,

    /// Backreference to the engine inner state so callbacks can manipulate
    /// the schedule map. Weak to avoid cycles.
    engine: Weak<EngineInner<K>>,
}

impl<K: Hash + Eq + Clone + 'static> Schedule<K> {
    fn dispatch(&self, ev: &str) {
        self.on.call(
            ev,
            &EventCtx {
                target: self.target.clone(),
                name: self.name.clone(),
                id: self.id,
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Engine inner
// ---------------------------------------------------------------------------

/// Map: target -> map of id -> schedule.
type ScheduleMap<K> = HashMap<K, HashMap<TransitionId, Rc<Schedule<K>>>>;

struct EngineInner<K: Hash + Eq + Clone + 'static> {
    timer_loop: TimerLoop,
    schedules: RefCell<ScheduleMap<K>>,
    next_id: Cell<u64>,
    /// Default easing applied to fresh transitions.
    default_ease: RefCell<Rc<dyn Fn(f64) -> f64>>,
    /// Default duration (ms) — d3 uses 250.
    default_duration: Cell<f64>,
    /// Default delay (ms) — d3 uses 0.
    default_delay: Cell<f64>,
}

// ---------------------------------------------------------------------------
// TransitionEngine
// ---------------------------------------------------------------------------

/// Engine managing the lifecycle of transitions on user-keyed targets.
///
/// Each engine owns an internal [`TimerLoop`]; you can also share an external
/// loop via [`TransitionEngine::with_timer_loop`] if you'd rather drive a
/// single loop for both timers and transitions.
#[derive(Clone)]
pub struct TransitionEngine<K: Hash + Eq + Clone + 'static> {
    inner: Rc<EngineInner<K>>,
}

impl<K: Hash + Eq + Clone + 'static> TransitionEngine<K> {
    /// Construct a new engine with a default [`InstantClock`].
    pub fn new() -> Self {
        Self::with_timer_loop(TimerLoop::new())
    }

    /// Construct an engine with a user-provided clock.
    pub fn with_clock<C: Clock + 'static>(clock: C) -> Self {
        Self::with_timer_loop(TimerLoop::with_clock(clock))
    }

    /// Construct an engine that shares the given [`TimerLoop`].
    pub fn with_timer_loop(timer_loop: TimerLoop) -> Self {
        TransitionEngine {
            inner: Rc::new(EngineInner {
                timer_loop,
                schedules: RefCell::new(HashMap::new()),
                next_id: Cell::new(1),
                default_ease: RefCell::new(Rc::new(cubic_in_out)),
                default_duration: Cell::new(250.0),
                default_delay: Cell::new(0.0),
            }),
        }
    }

    /// Returns the engine's underlying timer loop. Use this if you want to
    /// drive timers and transitions from a single tick.
    pub fn timer_loop(&self) -> &TimerLoop { &self.inner.timer_loop }

    /// Drive one tick at the engine's current clock time. Equivalent to
    /// `engine.timer_loop().tick()`.
    pub fn tick(&self) -> usize { self.inner.timer_loop.tick() }

    /// Returns the engine's clock time (ms). See [`TimerLoop::now`].
    pub fn now(&self) -> f64 { self.inner.timer_loop.now() }

    /// Override the default easing for newly-created transitions. d3 uses
    /// `cubicInOut` by default.
    pub fn set_default_ease<F: Fn(f64) -> f64 + 'static>(&self, ease: F) {
        *self.inner.default_ease.borrow_mut() = Rc::new(ease);
    }

    /// Override the default duration (ms). d3 uses 250.
    pub fn set_default_duration(&self, ms: f64) {
        self.inner.default_duration.set(ms);
    }

    /// Override the default delay (ms). d3 uses 0.
    pub fn set_default_delay(&self, ms: f64) {
        self.inner.default_delay.set(ms);
    }

    /// Number of *active* transitions across all targets (informational).
    pub fn len(&self) -> usize {
        self.inner
            .schedules
            .borrow()
            .values()
            .map(|m| m.len())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.schedules.borrow().is_empty()
    }

    /// Schedules a new transition on `target` with the given `name` (use an
    /// empty string for the default unnamed transition).
    ///
    /// Returns the [`TransitionId`] you'll use to attach tweens / event
    /// handlers / configure delay & duration. Mirrors d3's
    /// `selection.transition(name)` workflow.
    ///
    /// The transition is **not yet running** — its initial tick is scheduled
    /// at the loop's current time, and on that first tick it advances from
    /// `CREATED` → `SCHEDULED` → (after `delay`) → `STARTING` → `STARTED` →
    /// `RUNNING`. Configure delay/duration/ease *before* the first tick if
    /// you want non-default values; otherwise the defaults take effect.
    pub fn transition(&self, target: K, name: impl Into<String>) -> TransitionId {
        self.transition_at(target, name, None)
    }

    /// Like [`transition`], but allows specifying an absolute reference time.
    pub fn transition_at(
        &self,
        target: K,
        name: impl Into<String>,
        time: Option<f64>,
    ) -> TransitionId {
        let id_n = self.inner.next_id.get();
        self.inner.next_id.set(id_n + 1);
        let id = TransitionId(id_n);
        let now_ref = time.unwrap_or_else(|| self.inner.timer_loop.now());

        let on = Rc::new(Dispatch::<EventCtx<K>>::new(&[
            "start",
            "end",
            "cancel",
            "interrupt",
        ]));

        let schedule = Rc::new(Schedule {
            target: target.clone(),
            name: name.into(),
            id,
            time: now_ref,
            delay: Cell::new(self.inner.default_delay.get()),
            duration: Cell::new(self.inner.default_duration.get()),
            ease: RefCell::new({
                let e = self.inner.default_ease.borrow().clone();
                Box::new(move |t| (e)(t))
            }),
            tweens: RefCell::new(Vec::new()),
            state: Cell::new(State::Created),
            timer: RefCell::new(None),
            on,
            engine: Rc::downgrade(&self.inner),
        });

        // Insert into schedule map.
        {
            let mut all = self.inner.schedules.borrow_mut();
            let map = all.entry(target.clone()).or_default();
            map.insert(id, Rc::clone(&schedule));
        }

        // Kick off the per-schedule timer at delay=0, time=now_ref. The
        // first tick advances state->SCHEDULED and re-arms the timer at the
        // configured `delay`.
        let weak = Rc::downgrade(&schedule);
        let t = self
            .inner
            .timer_loop
            .timer(move |elapsed| Self::on_schedule(weak.clone(), elapsed), 0.0, Some(now_ref));
        *schedule.timer.borrow_mut() = Some(t);

        id
    }

    /// Look up a transition's current state. Returns `None` if the id is
    /// no longer tracked (already ended/cancelled and swept).
    pub fn state(&self, target: &K, id: TransitionId) -> Option<State> {
        let all = self.inner.schedules.borrow();
        all.get(target).and_then(|m| m.get(&id)).map(|s| s.state.get())
    }

    /// Set the delay for a transition. Equivalent to d3's `transition.delay()`.
    ///
    /// Throws via panic if the transition has already advanced past
    /// `CREATED` (mirrors d3's "too late; already scheduled").
    pub fn delay(&self, target: &K, id: TransitionId, delay_ms: f64) {
        let s = self.lookup(target, id).expect("transition not found");
        if s.state.get() > State::Created {
            panic!("too late; already scheduled");
        }
        s.delay.set(delay_ms);
    }

    /// Get the current delay value.
    pub fn get_delay(&self, target: &K, id: TransitionId) -> Option<f64> {
        self.lookup(target, id).map(|s| s.delay.get())
    }

    /// Set the duration. Allowed up to `STARTED` (mirrors d3's `set`).
    pub fn duration(&self, target: &K, id: TransitionId, duration_ms: f64) {
        let s = self.lookup(target, id).expect("transition not found");
        if s.state.get() > State::Started {
            panic!("too late; already running");
        }
        s.duration.set(duration_ms);
    }

    pub fn get_duration(&self, target: &K, id: TransitionId) -> Option<f64> {
        self.lookup(target, id).map(|s| s.duration.get())
    }

    /// Set the easing function. Allowed up to `STARTED`.
    pub fn ease<F: Fn(f64) -> f64 + 'static>(&self, target: &K, id: TransitionId, f: F) {
        let s = self.lookup(target, id).expect("transition not found");
        if s.state.get() > State::Started {
            panic!("too late; already running");
        }
        *s.ease.borrow_mut() = Box::new(f);
    }

    /// Add or replace a named tween. Allowed up to `STARTED` for a given
    /// schedule (matching d3's `schedule.tween` mutation guard).
    pub fn tween<F>(&self, target: &K, id: TransitionId, name: impl Into<String>, f: F)
    where
        F: FnMut(f64) + 'static,
    {
        let s = self.lookup(target, id).expect("transition not found");
        if s.state.get() > State::Started {
            panic!("too late; already running");
        }
        let name = name.into();
        let mut tweens = s.tweens.borrow_mut();
        let tw = Tween { name: name.clone(), value: Box::new(f) };
        for slot in tweens.iter_mut() {
            if slot.name == name {
                *slot = tw;
                return;
            }
        }
        tweens.push(tw);
    }

    /// Remove a named tween.
    pub fn remove_tween(&self, target: &K, id: TransitionId, name: &str) {
        let Some(s) = self.lookup(target, id) else { return };
        if s.state.get() > State::Started { return; }
        s.tweens.borrow_mut().retain(|t| t.name != name);
    }

    /// Subscribe to an event.
    ///
    /// Valid event names: `"start"`, `"end"`, `"cancel"`, `"interrupt"`.
    /// Optionally use a dot-namespace, e.g. `"end.cleanup"`.
    pub fn on<F>(&self, target: &K, id: TransitionId, event: &str, listener: F)
    where
        F: Fn(&EventCtx<K>) + 'static,
    {
        let s = self.lookup(target, id).expect("transition not found");
        s.on.on(event, Some(Rc::new(listener)));
    }

    /// Unsubscribe by typename / namespace. Pass `"end"` to remove the
    /// unnamed `end` listener, or `".tag"` to remove all listeners with that
    /// dot-namespace tag.
    pub fn off(&self, target: &K, id: TransitionId, event: &str) {
        if let Some(s) = self.lookup(target, id) {
            s.on.on(event, None);
        }
    }

    /// Interrupt every transition on the given target whose `name` matches.
    /// Pass `None` for `name` to match the unnamed default transition.
    /// Mirrors d3's `interrupt(node, name)`.
    pub fn interrupt(&self, target: &K, name: Option<&str>) {
        let to_interrupt: Vec<Rc<Schedule<K>>> = {
            let all = self.inner.schedules.borrow();
            let Some(map) = all.get(target) else { return };
            let want = name.unwrap_or("");
            map.values()
                .filter(|s| s.name == want)
                .cloned()
                .collect()
        };
        for s in to_interrupt {
            let active = s.state.get() > State::Starting && s.state.get() < State::Ending;
            s.state.set(State::Ended);
            if let Some(t) = s.timer.borrow().as_ref() { t.stop(); }
            s.dispatch(if active { "interrupt" } else { "cancel" });
            self.remove_schedule(&s.target, s.id);
        }
    }

    /// Find the *active* (post-`SCHEDULED`) transition on `target` with the
    /// given `name`, if any. Mirrors d3's `active(node, name)`.
    pub fn active(&self, target: &K, name: Option<&str>) -> Option<TransitionId> {
        let all = self.inner.schedules.borrow();
        let map = all.get(target)?;
        let want = name.unwrap_or("");
        map.values()
            .filter(|s| s.state.get() > State::Scheduled && s.name == want)
            .map(|s| s.id)
            .next()
    }

    // ----- internal helpers -----

    fn lookup(&self, target: &K, id: TransitionId) -> Option<Rc<Schedule<K>>> {
        let all = self.inner.schedules.borrow();
        all.get(target).and_then(|m| m.get(&id)).cloned()
    }

    fn remove_schedule(&self, target: &K, id: TransitionId) {
        let mut all = self.inner.schedules.borrow_mut();
        if let Some(map) = all.get_mut(target) {
            map.remove(&id);
            if map.is_empty() { all.remove(target); }
        }
    }

    // ----- state machine ------------------------------------------------

    /// Phase 1: scheduled. Equivalent to d3's `schedule(elapsed)` inner
    /// function — moves the state to SCHEDULED and re-arms the timer for
    /// the configured `delay`. If the elapsed delay is already past, fall
    /// straight through to start.
    fn on_schedule(weak: Weak<Schedule<K>>, elapsed: f64) {
        let Some(self_) = weak.upgrade() else { return };
        self_.state.set(State::Scheduled);

        // Re-arm the timer for the start phase: fire at delay (relative to
        // self.time). If delay <= elapsed, start immediately with the
        // overshoot.
        let delay = self_.delay.get();
        let weak_clone = weak.clone();
        if let Some(t) = self_.timer.borrow().as_ref() {
            t.restart(
                move |e| Self::on_start(weak_clone.clone(), e),
                delay,
                Some(self_.time),
            );
        }

        if delay <= elapsed {
            // Synchronously start with elapsed-delay overshoot.
            Self::on_start(weak, elapsed - delay);
        }
    }

    /// Phase 2: start. Equivalent to d3's `start(elapsed)`.
    fn on_start(weak: Weak<Schedule<K>>, elapsed: f64) {
        let Some(self_) = weak.upgrade() else { return };

        // If state isn't SCHEDULED any more, we're being called after a
        // previous error or cancel — drop on the floor.
        if self_.state.get() != State::Scheduled { return; }

        // Find the engine + sibling schedules so we can interrupt/cancel
        // peers per d3 semantics.
        let Some(engine_inner) = self_.engine.upgrade() else { return };

        // Walk all schedules on the same target. Take ownership of the list
        // before mutating since we'll be removing entries.
        let peers: Vec<Rc<Schedule<K>>> = {
            let all = engine_inner.schedules.borrow();
            match all.get(&self_.target) {
                Some(m) => m.values().cloned().collect(),
                None => return, // we were removed already
            }
        };

        let mut defer_due_to_started = false;

        for o in peers.iter() {
            if !Rc::ptr_eq(o, &self_) && o.name == self_.name {
                let s = o.state.get();
                if s == State::Started {
                    // d3: defer until that transition's tick has a chance
                    // to fire; we re-schedule via timer with a 0 delay.
                    defer_due_to_started = true;
                } else if s == State::Running {
                    // Interrupt the active transition.
                    o.state.set(State::Ended);
                    if let Some(t) = o.timer.borrow().as_ref() { t.stop(); }
                    o.dispatch("interrupt");
                    let mut all = engine_inner.schedules.borrow_mut();
                    if let Some(m) = all.get_mut(&o.target) {
                        m.remove(&o.id);
                    }
                } else if o.id < self_.id {
                    // Cancel pre-empted older transitions.
                    o.state.set(State::Ended);
                    if let Some(t) = o.timer.borrow().as_ref() { t.stop(); }
                    o.dispatch("cancel");
                    let mut all = engine_inner.schedules.borrow_mut();
                    if let Some(m) = all.get_mut(&o.target) {
                        m.remove(&o.id);
                    }
                }
            }
        }

        if defer_due_to_started {
            // Re-arm a 0-delay timer pointing at on_start; this gives the
            // already-STARTED peer a chance to advance.
            let weak2 = weak.clone();
            if let Some(t) = self_.timer.borrow().as_ref() {
                t.restart(
                    move |e| Self::on_start(weak2.clone(), e),
                    0.0,
                    None,
                );
            }
            return;
        }

        // Schedule the first tick. d3 also `timeout`s tick to the end of
        // the current frame; we approximate via a 0-delay restart, so that
        // tick fires on the *next* drive of the timer loop. After dispatch
        // ordering: first set state to STARTING, fire start event, advance
        // to STARTED, then restart with delay (so first tick is at the
        // active phase's start).
        self_.state.set(State::Starting);
        self_.dispatch("start");
        // If 'start' interrupted this transition (state changed to ENDED),
        // bail out.
        if self_.state.get() != State::Starting { return; }

        self_.state.set(State::Started);

        // Capture initial elapsed for the first tick — d3 immediately calls
        // tick(elapsed) after re-arming.
        let weak2 = weak.clone();
        if let Some(t) = self_.timer.borrow().as_ref() {
            t.restart(
                move |e| Self::on_tick(weak2.clone(), e),
                self_.delay.get(),
                Some(self_.time),
            );
        }

        // Run an immediate tick — d3 schedules a 0-delay timeout but for
        // pull-driven semantics we run synchronously here so callers don't
        // see a spurious "no-op" frame after 'start'.
        Self::on_tick(weak, elapsed);
    }

    /// Phase 3: running tick. Equivalent to d3's `tick(elapsed)`.
    fn on_tick(weak: Weak<Schedule<K>>, elapsed: f64) {
        let Some(self_) = weak.upgrade() else { return };
        if self_.state.get() < State::Started { return; }
        if self_.state.get() == State::Ended { return; }

        // Promote STARTED -> RUNNING the first time we tick.
        if self_.state.get() == State::Started {
            self_.state.set(State::Running);
        }

        let duration = self_.duration.get();
        let progress = if elapsed < duration {
            (self_.ease.borrow())(elapsed / duration)
        } else {
            // Final tick: stop the timer and ease(1).
            self_.state.set(State::Ending);
            if let Some(t) = self_.timer.borrow().as_ref() { t.stop(); }
            1.0
        };

        // Drive every tween. We bundle the borrow into a separate scope so
        // tween callbacks can call back into the engine (e.g. to schedule a
        // chained transition) without re-borrow conflicts.
        let mut tweens = self_.tweens.borrow_mut();
        for tw in tweens.iter_mut() {
            (tw.value)(progress);
        }
        drop(tweens);

        if self_.state.get() == State::Ending {
            self_.dispatch("end");
            self_.state.set(State::Ended);
            if let Some(engine_inner) = self_.engine.upgrade() {
                let mut all = engine_inner.schedules.borrow_mut();
                if let Some(m) = all.get_mut(&self_.target) {
                    m.remove(&self_.id);
                    if m.is_empty() {
                        all.remove(&self_.target);
                    }
                }
            }
        }
    }
}

impl<K: Hash + Eq + Clone + 'static> Default for TransitionEngine<K> {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ease::linear;
    use crate::timer::ManualClock;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn engine_with_clock() -> (TransitionEngine<u64>, Rc<ManualClock>) {
        struct ClockProxy(Rc<ManualClock>);
        impl Clock for ClockProxy {
            fn now_ms(&self) -> f64 { self.0.now_ms() }
        }
        let mc = Rc::new(ManualClock::new(0.0));
        let eng = TransitionEngine::with_clock(ClockProxy(Rc::clone(&mc)));
        (eng, mc)
    }

    #[test]
    fn transition_creates_and_progresses() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(100.0);
        let id = eng.transition(1u64, "");
        // Tween records progress values
        let log: Rc<RefCell<Vec<f64>>> = Rc::new(RefCell::new(Vec::new()));
        let l = log.clone();
        eng.tween(&1, id, "v", move |t| l.borrow_mut().push(t));
        eng.ease(&1, id, linear);
        assert_eq!(eng.state(&1, id), Some(State::Created));

        // Tick at t=0 -> schedule + start synchronously, plus initial tick at progress=0
        mc.set(0.0); eng.tick();
        // After first tick we should be STARTED or RUNNING with progress emitted.
        let v = log.borrow().clone();
        assert!(!v.is_empty(), "tween should have fired");
        assert!((v[0] - 0.0).abs() < 1e-9);

        // Advance halfway
        mc.set(50.0); eng.tick();
        let v = log.borrow().clone();
        assert!(v.iter().any(|&x| (x - 0.5).abs() < 1e-6), "expected ~0.5: {v:?}");

        // Finish
        mc.set(100.0); eng.tick();
        let v = log.borrow().clone();
        assert!(v.last().map(|&x| (x - 1.0).abs() < 1e-9).unwrap_or(false), "last={:?}", v.last());
        // After ending, schedule should be swept
        assert_eq!(eng.state(&1, id), None);
    }

    #[test]
    fn delay_postpones_start() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(100.0);
        let id = eng.transition(1u64, "");
        eng.delay(&1, id, 50.0);
        let started = Rc::new(Cell::new(false));
        let s = started.clone();
        eng.on(&1, id, "start", move |_| s.set(true));

        mc.set(0.0); eng.tick();
        assert!(!started.get(), "should not start before delay elapses");

        mc.set(40.0); eng.tick();
        assert!(!started.get());

        mc.set(50.0); eng.tick();
        assert!(started.get());
    }

    #[test]
    fn end_event_fires_once() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(50.0);
        let id = eng.transition(1u64, "");
        eng.ease(&1, id, linear);
        let ends = Rc::new(Cell::new(0u32));
        let e = ends.clone();
        eng.on(&1, id, "end", move |_| e.set(e.get() + 1));

        mc.set(0.0); eng.tick();
        mc.set(25.0); eng.tick();
        mc.set(50.0); eng.tick();
        mc.set(75.0); eng.tick();
        assert_eq!(ends.get(), 1);
    }

    #[test]
    fn newer_transition_interrupts_running_one() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(100.0);

        let id1 = eng.transition(1u64, "x");
        eng.ease(&1, id1, linear);
        // Start id1 + advance into RUNNING
        mc.set(0.0); eng.tick();
        mc.set(50.0); eng.tick();
        assert_eq!(eng.state(&1, id1), Some(State::Running));

        let interrupts = Rc::new(Cell::new(0u32));
        let i = interrupts.clone();
        eng.on(&1, id1, "interrupt", move |_| i.set(i.get() + 1));

        let id2 = eng.transition(1u64, "x");
        // Tick advances id2 into starting -> interrupts id1
        mc.set(60.0); eng.tick();
        assert_eq!(interrupts.get(), 1);
        assert_eq!(eng.state(&1, id1), None);
        // id2 should be running
        let s2 = eng.state(&1, id2);
        assert!(matches!(s2, Some(State::Running) | Some(State::Started)), "got {s2:?}");
    }

    #[test]
    fn new_transition_cancels_older_pre_started() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(100.0);
        // Both transitions with same name, same target; id2 newer.
        let id1 = eng.transition(1u64, "x");
        // Set delay so it stays SCHEDULED for a while
        eng.delay(&1, id1, 1000.0);

        let id2 = eng.transition(1u64, "x");
        // id2 has default delay 0 -> will start at next tick

        let cancels = Rc::new(Cell::new(0u32));
        let c = cancels.clone();
        eng.on(&1, id1, "cancel", move |_| c.set(c.get() + 1));

        mc.set(0.0); eng.tick();
        // id1 should be cancelled
        assert_eq!(cancels.get(), 1);
        assert_eq!(eng.state(&1, id1), None);
        // id2 should be running/started
        let s2 = eng.state(&1, id2);
        assert!(matches!(s2, Some(State::Running) | Some(State::Started) | Some(State::Starting)));
    }

    #[test]
    fn interrupt_by_name() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(100.0);
        let id_a = eng.transition(1u64, "a");
        let id_b = eng.transition(1u64, "b");

        // Move into RUNNING
        mc.set(0.0); eng.tick();
        mc.set(10.0); eng.tick();

        let inter_a = Rc::new(Cell::new(0u32));
        let inter_b = Rc::new(Cell::new(0u32));
        let ia = inter_a.clone(); let ib = inter_b.clone();
        eng.on(&1, id_a, "interrupt", move |_| ia.set(ia.get() + 1));
        eng.on(&1, id_b, "interrupt", move |_| ib.set(ib.get() + 1));

        // Interrupt only "a"
        eng.interrupt(&1, Some("a"));
        assert_eq!(inter_a.get(), 1);
        assert_eq!(inter_b.get(), 0);
        assert_eq!(eng.state(&1, id_a), None);
        assert!(eng.state(&1, id_b).is_some());
    }

    #[test]
    fn active_returns_currently_running_id() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(100.0);
        let id = eng.transition(1u64, "go");
        assert_eq!(eng.active(&1, Some("go")), None); // not started yet
        mc.set(0.0); eng.tick();
        // After first tick, transition is past SCHEDULED.
        assert_eq!(eng.active(&1, Some("go")), Some(id));
    }

    #[test]
    fn cannot_set_delay_after_scheduled() {
        let (eng, mc) = engine_with_clock();
        let id = eng.transition(1u64, "");
        mc.set(0.0); eng.tick(); // advances state past CREATED
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            eng.delay(&1, id, 10.0);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn cannot_set_duration_after_running() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(100.0);
        let id = eng.transition(1u64, "");
        mc.set(0.0); eng.tick();
        mc.set(20.0); eng.tick(); // RUNNING
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            eng.duration(&1, id, 50.0);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn tween_replace_preserves_position() {
        let (eng, _mc) = engine_with_clock();
        let id = eng.transition(1u64, "");
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let (l1, l2, l3) = (log.clone(), log.clone(), log.clone());
        eng.tween(&1, id, "a", move |_| l1.borrow_mut().push("A"));
        eng.tween(&1, id, "b", move |_| l2.borrow_mut().push("B"));
        // Replace 'a' — should remain at position 0
        eng.tween(&1, id, "a", move |_| l3.borrow_mut().push("a"));

        // We can't directly drive the tween order without ticking; emulate via
        // duration=0 to fire one tick and observe order.
        eng.duration(&1, id, 0.0);
        let mc = Rc::new(ManualClock::new(0.0));
        let _ = mc; // we already created the engine; reuse the engine's clock by ticking
        // Use the engine-owned clock by ticking at the default clock:
        eng.tick();

        let v = log.borrow().clone();
        // 'a' (replacement) appears before 'B'
        let pos_a = v.iter().position(|&s| s == "a");
        let pos_b = v.iter().position(|&s| s == "B");
        if let (Some(a), Some(b)) = (pos_a, pos_b) { assert!(a < b, "{:?}", v); }
    }

    #[test]
    fn off_unsubscribes() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(50.0);
        let id = eng.transition(1u64, "");
        eng.ease(&1, id, linear);
        let ends = Rc::new(Cell::new(0u32));
        let e = ends.clone();
        eng.on(&1, id, "end.foo", move |_| e.set(e.get() + 1));
        eng.off(&1, id, ".foo");
        mc.set(0.0); eng.tick();
        mc.set(50.0); eng.tick();
        assert_eq!(ends.get(), 0);
    }

    #[test]
    fn remove_tween_works() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(50.0);
        let id = eng.transition(1u64, "");
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        eng.tween(&1, id, "a", move |_| c.set(c.get() + 1));
        eng.remove_tween(&1, id, "a");
        eng.ease(&1, id, linear);
        mc.set(0.0); eng.tick();
        mc.set(25.0); eng.tick();
        mc.set(50.0); eng.tick();
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn many_transitions_independent_targets() {
        let (eng, mc) = engine_with_clock();
        eng.set_default_duration(50.0);
        let id1 = eng.transition(1u64, "");
        let id2 = eng.transition(2u64, "");
        let id3 = eng.transition(3u64, "");
        mc.set(0.0); eng.tick();
        // All three should advance independently (no interrupts between targets)
        assert!(eng.state(&1, id1).is_some());
        assert!(eng.state(&2, id2).is_some());
        assert!(eng.state(&3, id3).is_some());
        mc.set(50.0); eng.tick();
        // All ended
        assert_eq!(eng.state(&1, id1), None);
        assert_eq!(eng.state(&2, id2), None);
        assert_eq!(eng.state(&3, id3), None);
    }
}
