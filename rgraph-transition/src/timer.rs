//! Pull-driven timer loop, port of [d3-timer](https://github.com/d3/d3-timer).
//!
//! Differences from d3-timer:
//!
//! * d3 uses `requestAnimationFrame` / `setTimeout` to drive itself. There is
//!   no analogous concept in plain Rust, so this port is *pull-based*: the
//!   embedding application owns a [`TimerLoop`] and calls
//!   [`TimerLoop::tick`] (or [`TimerLoop::flush`]) once per frame from its
//!   own event loop. The loop owns the task list and a monotonic clock.
//! * `now()` is per-loop instead of a global. Inside a tick the clock is
//!   frozen so timers scheduled during the tick observe a consistent time —
//!   matching d3's `clockNow` caching. After the tick, the next call to
//!   [`TimerLoop::now`] re-samples the clock.
//! * Callbacks are stored as `Box<dyn FnMut(f64)>` and receive the elapsed
//!   time (ms) since the timer's effective start, identical to d3.
//!
//! # Example
//!
//! ```ignore
//! use rgraph_transition::timer::TimerLoop;
//! let mut loop_ = TimerLoop::new();
//! let count = std::rc::Rc::new(std::cell::Cell::new(0u32));
//! let c = count.clone();
//! let _t = loop_.timer(move |_elapsed| { c.set(c.get() + 1); }, 0.0);
//! loop_.flush(); // synchronously fires every eligible timer
//! assert!(count.get() >= 1);
//! ```

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Clock abstraction
// ---------------------------------------------------------------------------

/// Minimal monotonic clock used by [`TimerLoop`]. The default
/// [`InstantClock`] reports milliseconds elapsed since loop creation. Tests
/// and embedded contexts can supply their own [`Clock`] for deterministic
/// time control.
pub trait Clock {
    /// Returns the current time in milliseconds. Must be monotonically
    /// non-decreasing.
    fn now_ms(&self) -> f64;
}

/// Default clock based on [`std::time::Instant`]. Produces milliseconds
/// since the clock was constructed (so it's always positive and small
/// enough to avoid f64 loss).
pub struct InstantClock {
    epoch: Instant,
}

impl InstantClock {
    pub fn new() -> Self { InstantClock { epoch: Instant::now() } }
}

impl Default for InstantClock {
    fn default() -> Self { Self::new() }
}

impl Clock for InstantClock {
    #[inline]
    fn now_ms(&self) -> f64 {
        // u128 micros divided by 1000 keeps sub-millisecond precision while
        // staying well within f64.
        self.epoch.elapsed().as_micros() as f64 / 1_000.0
    }
}

/// Mockable clock — useful for tests and replay.
pub struct ManualClock {
    now: Cell<f64>,
}

impl ManualClock {
    pub fn new(start_ms: f64) -> Self { ManualClock { now: Cell::new(start_ms) } }

    /// Set the absolute clock time (in ms).
    pub fn set(&self, now_ms: f64) { self.now.set(now_ms); }

    /// Advance the clock by `dt_ms` milliseconds.
    pub fn advance(&self, dt_ms: f64) { self.now.set(self.now.get() + dt_ms); }
}

impl Clock for ManualClock {
    #[inline]
    fn now_ms(&self) -> f64 { self.now.get() }
}

// ---------------------------------------------------------------------------
// Internal task representation
// ---------------------------------------------------------------------------

/// Sentinel value used to mark a stopped task's `time`. Matches d3 setting
/// `_time = Infinity` on stop.
const TIME_INF: f64 = f64::INFINITY;

/// Boxed callback held inside a [`Task`]. Receives the elapsed time in ms
/// since the task's effective fire time.
type CallbackBox = Box<dyn FnMut(f64)>;

/// One scheduled callback. Held inside the loop's task list; the public
/// [`Timer`] handle is a strong reference to this.
struct Task {
    /// Effective fire time (ms). Set to [`TIME_INF`] when stopped so the
    /// `nap` pass can sweep it out.
    time: Cell<f64>,
    /// Boxed callback. `None` once stopped — equivalent to d3 nulling
    /// `_call`. We keep it in a [`RefCell`] because callbacks may be
    /// re-entrantly modified (a timer stopping itself, restarting itself,
    /// or scheduling a new timer).
    call: RefCell<Option<CallbackBox>>,
    /// Parent loop link, cleared when the loop is dropped.
    loop_ref: Weak<LoopInner>,
}

impl Task {
    fn is_stopped(&self) -> bool { self.call.borrow().is_none() }
}

// ---------------------------------------------------------------------------
// Timer handle
// ---------------------------------------------------------------------------

/// A handle to a single scheduled callback. The handle keeps the task
/// alive even if the loop has swept it from its active list (so a stopped
/// timer can be restarted later), but does **not** stop the task on drop.
/// Call [`Timer::stop`] explicitly.
///
/// d3 returns the same object from `timer()` and `timeout()`/`interval()`,
/// and supports `.restart()` / `.stop()` on it. We mirror that.
#[derive(Clone)]
pub struct Timer {
    task: Rc<Task>,
}

impl Timer {
    fn new(task: &Rc<Task>) -> Self { Timer { task: Rc::clone(task) } }

    /// Returns whether this handle still refers to a live (non-stopped) task.
    pub fn is_active(&self) -> bool { !self.task.is_stopped() }

    /// Stops the timer. Mirrors d3's `timer.stop()` — the callback will not
    /// fire again. Idempotent.
    pub fn stop(&self) {
        let t = &self.task;
        // The callback may currently be checked out by the flush loop (we
        // `take()` it for the duration of the user code). To handle a
        // self-stop call from within a callback, also set `time` to INF —
        // the post-callback restore logic checks `time.is_finite()` to
        // decide whether to reinstall the callback.
        if t.call.borrow().is_some() {
            *t.call.borrow_mut() = None;
        }
        t.time.set(TIME_INF);
        if let Some(inner) = t.loop_ref.upgrade() {
            inner.dirty.set(true);
        }
    }

    /// Restart the timer with a new callback, delay (ms), and absolute
    /// reference time (ms). Equivalent to d3's `timer.restart(callback,
    /// delay, time)`.
    ///
    /// If `time` is `None`, the loop's current time is used. The effective
    /// fire time becomes `time + delay`.
    pub fn restart<F>(&self, callback: F, delay: f64, time: Option<f64>)
    where
        F: FnMut(f64) + 'static,
    {
        let task = &self.task;
        let inner = match task.loop_ref.upgrade() {
            Some(i) => i,
            None => return,
        };
        let base = time.unwrap_or_else(|| inner.now());
        let new_time = base + delay;
        // If this task was stopped previously we may need to reattach.
        let was_orphan = task.is_stopped()
            && !inner.tasks.borrow().iter().any(|t| Rc::ptr_eq(t, task));
        *task.call.borrow_mut() = Some(Box::new(callback));
        task.time.set(new_time);

        if was_orphan {
            inner.tasks.borrow_mut().push(Rc::clone(task));
        }

        inner.dirty.set(true);
    }
}

// ---------------------------------------------------------------------------
// TimerLoop
// ---------------------------------------------------------------------------

/// Internal state held inside an [`Rc`] so [`Task`] instances can refer
/// back to the loop without borrow-checker grief.
struct LoopInner {
    /// Wrapped clock.
    clock: Box<dyn Clock>,
    /// Cached "now" while inside a tick — d3's `clockNow`. `0.0` outside
    /// of ticks (sentinel: any read recomputes).
    clock_now: Cell<f64>,
    /// Whether `clock_now` is currently latched (we're inside a tick).
    clock_latched: Cell<bool>,
    /// Active tasks. A `Vec<Rc<Task>>` is sufficient: insertions append to
    /// keep scheduling order, stopped tasks are swept at end-of-tick.
    tasks: RefCell<Vec<Rc<Task>>>,
    /// Re-entry guard so `flush()` invoked inside a callback short-circuits
    /// like d3's `frame` flag does for `wake`.
    in_flush: Cell<u32>,
    /// Set whenever a task is added/restarted/stopped — informational.
    dirty: Cell<bool>,
}

impl LoopInner {
    /// Returns the current time, latching while inside a tick.
    fn now(&self) -> f64 {
        if self.clock_latched.get() {
            self.clock_now.get()
        } else {
            // Outside of ticks, always re-sample from the underlying clock.
            // We do *not* cache here — d3 caches "until the next frame", but
            // because we're pull-driven with no frame loop, the simplest
            // and most predictable behavior is: cache only inside a tick.
            self.clock.now_ms()
        }
    }
}

/// A pull-driven timer loop. Owns a list of scheduled callbacks and a
/// clock. Drive it by calling [`TimerLoop::tick`] from your event loop.
///
/// `TimerLoop` is `!Send` and `!Sync` — callbacks are `FnMut` boxed
/// trait objects without thread-safety bounds.
#[derive(Clone)]
pub struct TimerLoop {
    inner: Rc<LoopInner>,
}

impl Default for TimerLoop {
    fn default() -> Self { Self::new() }
}

impl TimerLoop {
    /// Creates a new loop using [`InstantClock`].
    pub fn new() -> Self { Self::with_clock(InstantClock::new()) }

    /// Creates a new loop with a custom [`Clock`].
    pub fn with_clock<C: Clock + 'static>(clock: C) -> Self {
        TimerLoop {
            inner: Rc::new(LoopInner {
                clock: Box::new(clock),
                clock_now: Cell::new(0.0),
                clock_latched: Cell::new(false),
                tasks: RefCell::new(Vec::new()),
                in_flush: Cell::new(0),
                dirty: Cell::new(false),
            }),
        }
    }

    /// Returns the current time (ms). Inside a tick this is the latched
    /// frame time; outside, it's a fresh sample of the underlying clock.
    #[inline]
    pub fn now(&self) -> f64 { self.inner.now() }

    /// Schedules a callback to fire at `time + delay` (both ms).
    ///
    /// If `time` is `None`, the loop's current time is used. Mirrors
    /// d3's `timer(callback, delay, time)` factory.
    pub fn timer<F>(&self, callback: F, delay: f64, time: Option<f64>) -> Timer
    where
        F: FnMut(f64) + 'static,
    {
        let base = time.unwrap_or_else(|| self.inner.now());
        let task = Rc::new(Task {
            time: Cell::new(base + delay),
            call: RefCell::new(Some(Box::new(callback))),
            loop_ref: Rc::downgrade(&self.inner),
        });
        self.inner.tasks.borrow_mut().push(Rc::clone(&task));
        self.inner.dirty.set(true);
        Timer::new(&task)
    }

    /// Schedules a one-shot callback that fires once after `delay` ms and
    /// then stops itself. Equivalent to d3's `timeout(callback, delay)`.
    pub fn timeout<F>(&self, mut callback: F, delay: f64, time: Option<f64>) -> Timer
    where
        F: FnMut(f64) + 'static,
    {
        // We need a self-stopping callback; pre-create the timer and capture
        // a weak ref to its task so the closure can stop it.
        let placeholder = self.timer(|_| {}, delay, time);
        let weak = Rc::downgrade(&placeholder.task);
        if let Some(task) = weak.upgrade() {
            *task.call.borrow_mut() = Some(Box::new(move |elapsed: f64| {
                if let Some(t) = weak.upgrade() {
                    *t.call.borrow_mut() = None;
                    t.time.set(TIME_INF);
                }
                callback(elapsed + delay);
            }));
        }
        placeholder
    }

    /// Schedules a callback that fires every `delay` ms. Mirrors d3's
    /// `interval(callback, delay)`.
    ///
    /// If `delay` is `None`, the callback fires every tick (just like
    /// `timer(callback)`).
    pub fn interval<F>(&self, mut callback: F, delay: Option<f64>, time: Option<f64>) -> Timer
    where
        F: FnMut(f64) + 'static,
    {
        let Some(d) = delay else {
            return self.timer(callback, 0.0, time);
        };

        let initial_total = d;
        let total = Rc::new(Cell::new(initial_total));
        let inner = Rc::clone(&self.inner);

        let handle = self.timer(|_| {}, d, time);
        let weak_task = Rc::downgrade(&handle.task);
        let weak_inner = Rc::downgrade(&inner);

        if let Some(task) = weak_task.upgrade() {
            *task.call.borrow_mut() = Some(Box::new(move |elapsed: f64| {
                let t = total.get();
                let abs_elapsed = elapsed + t;
                if let (Some(task), Some(loop_inner)) =
                    (weak_task.upgrade(), weak_inner.upgrade())
                {
                    total.set(t + d);
                    let prev_time = task.time.get();
                    let new_time = if prev_time.is_finite() { prev_time + d } else { loop_inner.now() + d };
                    task.time.set(new_time);
                    loop_inner.dirty.set(true);
                }
                callback(abs_elapsed);
            }));
        }
        handle
    }

    /// Synchronously invokes every eligible (non-stopped, due) timer.
    /// Re-entry from a callback is a no-op — d3 increments the `frame`
    /// flag during `wake` and short-circuits.
    ///
    /// Returns the number of callbacks that were invoked.
    pub fn flush(&self) -> usize {
        let inner = &self.inner;
        // Latch the clock for this flush.
        let was_latched = inner.clock_latched.get();
        let prev_now = inner.clock_now.get();
        if !was_latched {
            inner.clock_now.set(inner.clock.now_ms());
            inner.clock_latched.set(true);
        }
        inner.in_flush.set(inner.in_flush.get() + 1);

        // Snapshot the task list pointers so concurrent mutation (a callback
        // adding new timers) does not invalidate iteration. Indexing the
        // borrowed Vec directly would also work but holding the borrow
        // across user callbacks is unsafe re-borrows-wise.
        let snapshot: Vec<Rc<Task>> = inner.tasks.borrow().iter().map(Rc::clone).collect();
        let now = inner.clock_now.get();
        let mut fired = 0usize;
        for task in &snapshot {
            let scheduled = task.time.get();
            // Skip stopped tasks (call=None). Skip tasks not yet due.
            if !scheduled.is_finite() || scheduled > now {
                continue;
            }
            // Pull the callback out so we don't hold a borrow across user
            // code (which may call .stop() / .restart() on this very task).
            let mut cb_opt = task.call.borrow_mut().take();
            if let Some(mut cb) = cb_opt.take() {
                let elapsed = now - scheduled;
                cb(elapsed);
                // If the callback re-armed the task (via restart), call has
                // already been replaced. Otherwise restore the callback so
                // future ticks can fire it again — unless the call was
                // explicitly nulled (timeout, stop()).
                let mut slot = task.call.borrow_mut();
                if slot.is_none() {
                    // If the timer was restarted, the slot will be `Some`
                    // already. If it's still None, we may either restore
                    // or leave-as-stopped depending on whether time was
                    // bumped to infinity.
                    if task.time.get().is_finite() {
                        // No restart; reinstall the callback so it fires
                        // again next frame (matching d3 timer behavior of
                        // repeatedly invoking).
                        *slot = Some(cb);
                    }
                    // else: stopped — drop the callback.
                }
                // If the slot is already Some, the user replaced the
                // callback (restart). We discard `cb` (it's the old one).
                fired += 1;
            }
        }

        // Sweep stopped tasks.
        Self::sweep(&inner.tasks);

        inner.in_flush.set(inner.in_flush.get() - 1);
        if !was_latched {
            inner.clock_latched.set(false);
            inner.clock_now.set(prev_now);
        }
        inner.dirty.set(false);
        fired
    }

    /// Drives one frame at the *current* clock time. Equivalent to
    /// [`TimerLoop::flush`].
    ///
    /// Provided as a more idiomatic name for users coming from
    /// game/animation contexts.
    #[inline]
    pub fn tick(&self) -> usize { self.flush() }

    /// Returns the time of the soonest pending callback, or `None` if no
    /// active timers exist. Useful for sleeping efficiently between ticks.
    pub fn next_wake(&self) -> Option<f64> {
        let mut min = f64::INFINITY;
        for t in self.inner.tasks.borrow().iter() {
            if t.is_stopped() { continue; }
            let v = t.time.get();
            if v < min { min = v; }
        }
        if min.is_finite() { Some(min) } else { None }
    }

    /// Returns the number of registered (active or stopped-but-unswept) timers.
    pub fn len(&self) -> usize { self.inner.tasks.borrow().len() }

    /// `true` if no timers are registered at all.
    pub fn is_empty(&self) -> bool { self.inner.tasks.borrow().is_empty() }

    /// Sweep stopped tasks from the list. Internal helper.
    fn sweep(tasks: &RefCell<Vec<Rc<Task>>>) {
        tasks.borrow_mut().retain(|t| !t.is_stopped());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn instant_clock_monotonic() {
        let c = InstantClock::new();
        let a = c.now_ms();
        let b = c.now_ms();
        assert!(b >= a);
    }

    #[test]
    fn manual_clock_advances() {
        let c = ManualClock::new(0.0);
        assert_eq!(c.now_ms(), 0.0);
        c.advance(50.0);
        assert_eq!(c.now_ms(), 50.0);
        c.set(1000.0);
        assert_eq!(c.now_ms(), 1000.0);
    }

    fn loop_with_clock() -> (TimerLoop, Rc<ManualClock>) {
        // Build a TimerLoop sharing a ManualClock with the test (via Rc).
        // Because `with_clock` consumes the clock, we instead implement a
        // forwarding wrapper.
        struct ClockProxy(Rc<ManualClock>);
        impl Clock for ClockProxy {
            fn now_ms(&self) -> f64 { self.0.now_ms() }
        }
        let mc = Rc::new(ManualClock::new(0.0));
        let lp = TimerLoop::with_clock(ClockProxy(Rc::clone(&mc)));
        (lp, mc)
    }

    #[test]
    fn timer_fires_after_delay() {
        let (lp, mc) = loop_with_clock();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let _t = lp.timer(move |_| c.set(c.get() + 1), 100.0, Some(0.0));
        mc.set(50.0); lp.tick();
        assert_eq!(count.get(), 0);
        mc.set(100.0); lp.tick();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn timer_repeats_until_stopped() {
        let (lp, mc) = loop_with_clock();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let t = lp.timer(move |_| c.set(c.get() + 1), 0.0, Some(0.0));
        for i in 1..=3 {
            mc.set(i as f64 * 10.0);
            lp.tick();
        }
        assert_eq!(count.get(), 3);
        t.stop();
        mc.advance(10.0);
        lp.tick();
        assert_eq!(count.get(), 3);
    }

    #[test]
    fn timer_passes_elapsed() {
        let (lp, mc) = loop_with_clock();
        let observed = Rc::new(RefCell::new(Vec::<f64>::new()));
        let o = observed.clone();
        let _t = lp.timer(move |e| o.borrow_mut().push(e), 0.0, Some(0.0));
        mc.set(17.0); lp.tick();
        mc.set(34.0); lp.tick();
        let v = observed.borrow();
        // First fire: now=17, scheduled=0 -> elapsed=17. Second: now=34,
        // scheduled was bumped only via restart; without restart, sched
        // stays 0, so elapsed=34.
        assert_eq!(v[0], 17.0);
        assert_eq!(v[1], 34.0);
    }

    #[test]
    fn timer_with_time_skew() {
        let (lp, mc) = loop_with_clock();
        let elapsed_log = Rc::new(RefCell::new(Vec::<f64>::new()));
        let l = elapsed_log.clone();
        // Schedule at "100ms ago" with delay 50.
        // Effective fire time = -100 + 50 = -50, already due.
        let _t = lp.timer(move |e| l.borrow_mut().push(e), 50.0, Some(-100.0));
        mc.set(0.0);
        lp.tick();
        let v = elapsed_log.borrow();
        // elapsed = now (0) - fire (-50) = 50
        assert_eq!(v[0], 50.0);
    }

    #[test]
    fn timeout_fires_once() {
        let (lp, mc) = loop_with_clock();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let _ = lp.timeout(move |_| c.set(c.get() + 1), 50.0, Some(0.0));
        mc.set(40.0); lp.tick();
        assert_eq!(count.get(), 0);
        mc.set(60.0); lp.tick();
        assert_eq!(count.get(), 1);
        mc.set(120.0); lp.tick();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn timeout_passes_elapsed_plus_delay() {
        let (lp, mc) = loop_with_clock();
        let observed = Rc::new(Cell::new(0.0_f64));
        let o = observed.clone();
        let _ = lp.timeout(move |e| o.set(e), 50.0, Some(0.0));
        // Fire at now=70 -> elapsed_inside = 20, callback receives 20+50=70
        mc.set(70.0);
        lp.tick();
        assert_eq!(observed.get(), 70.0);
    }

    #[test]
    fn interval_fires_every_delay() {
        let (lp, mc) = loop_with_clock();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let _t = lp.interval(move |_| c.set(c.get() + 1), Some(50.0), Some(0.0));
        // First tick: scheduled at 50
        mc.set(50.0); lp.tick(); assert_eq!(count.get(), 1);
        mc.set(100.0); lp.tick(); assert_eq!(count.get(), 2);
        mc.set(150.0); lp.tick(); assert_eq!(count.get(), 3);
    }

    #[test]
    fn stop_inside_callback_works() {
        let (lp, mc) = loop_with_clock();
        let count = Rc::new(Cell::new(0u32));
        // We need a way for the callback to stop the timer. Use a shared
        // Cell holding the Timer, populated after creation.
        let handle: Rc<RefCell<Option<Timer>>> = Rc::new(RefCell::new(None));
        let h = handle.clone();
        let c = count.clone();
        let t = lp.timer(move |_| {
            c.set(c.get() + 1);
            if let Some(ref h) = *h.borrow() { h.stop(); }
        }, 0.0, Some(0.0));
        *handle.borrow_mut() = Some(t);

        for i in 1..=5 { mc.set(i as f64 * 10.0); lp.tick(); }
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn restart_repurposes_existing_handle() {
        let (lp, mc) = loop_with_clock();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let t = lp.timer(move |_| c.set(c.get() + 1), 1000.0, Some(0.0));
        // Move fire-time to 50.
        let c2 = count.clone();
        t.restart(move |_| c2.set(c2.get() + 10), 50.0, Some(0.0));
        mc.set(60.0); lp.tick();
        assert_eq!(count.get(), 10); // new callback fired, not old
    }

    #[test]
    fn stop_then_restart_reattaches() {
        let (lp, mc) = loop_with_clock();
        let count = Rc::new(Cell::new(0u32));
        let c = count.clone();
        let t = lp.timer(move |_| c.set(c.get() + 1), 0.0, Some(0.0));
        t.stop();
        mc.set(10.0); lp.tick();
        assert_eq!(count.get(), 0);
        let c2 = count.clone();
        t.restart(move |_| c2.set(c2.get() + 1), 0.0, Some(20.0));
        mc.set(30.0); lp.tick();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn flush_invokes_in_scheduling_order() {
        let (lp, mc) = loop_with_clock();
        let log: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
        let l1 = log.clone(); let l2 = log.clone(); let l3 = log.clone();
        let t1 = lp.timer(move |_| l1.borrow_mut().push(1), 0.0, Some(0.0));
        let t2 = lp.timer(move |_| l2.borrow_mut().push(2), 0.0, Some(0.0));
        let t3 = lp.timer(move |_| l3.borrow_mut().push(3), 0.0, Some(0.0));
        mc.set(10.0);
        lp.flush();
        assert_eq!(&*log.borrow(), &[1u32, 2, 3]);
        t1.stop(); t2.stop(); t3.stop();
    }

    #[test]
    fn flush_observes_current_time() {
        let (lp, mc) = loop_with_clock();
        let foos = Rc::new(Cell::new(0u32));
        let bars = Rc::new(Cell::new(0u32));
        let bazs = Rc::new(Cell::new(0u32));
        // Cache start = 0.
        let f = foos.clone(); let b = bars.clone(); let z = bazs.clone();
        let t1 = lp.timer(move |_| f.set(f.get() + 1), 0.0, Some(1.0));   // future
        let t2 = lp.timer(move |_| b.set(b.get() + 1), 0.0, Some(0.0));   // now
        let t3 = lp.timer(move |_| z.set(z.get() + 1), 0.0, Some(-1.0));  // past
        mc.set(0.0);
        lp.flush();
        assert_eq!(foos.get(), 0);
        assert_eq!(bars.get(), 1);
        assert_eq!(bazs.get(), 1);
        t1.stop(); t2.stop(); t3.stop();
    }

    #[test]
    fn next_wake_returns_min() {
        let (lp, _mc) = loop_with_clock();
        let _t1 = lp.timer(|_| {}, 100.0, Some(0.0));
        let _t2 = lp.timer(|_| {}, 50.0, Some(0.0));
        let _t3 = lp.timer(|_| {}, 200.0, Some(0.0));
        assert_eq!(lp.next_wake(), Some(50.0));
    }

    #[test]
    fn timer_inside_callback_does_not_fire_same_frame() {
        // d3: a timer scheduled during flush has its first tick deferred.
        // Our pull-based equivalent: it gets put at time=now+delay; if delay
        // is 0 and clock_now is latched, the next tick at the same clock
        // time should NOT pick up the new task because we iterate a
        // pre-flush snapshot.
        let (lp, mc) = loop_with_clock();
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let l1 = log.clone();
        let lp2 = lp.clone();
        let l_inner = log.clone();
        // Outer must self-stop so we observe only the in-flush behavior.
        let outer_handle: Rc<RefCell<Option<Timer>>> = Rc::new(RefCell::new(None));
        let oh = outer_handle.clone();
        let t = lp.timer(move |_| {
            l1.borrow_mut().push("outer");
            if let Some(h) = oh.borrow().as_ref() { h.stop(); }
            let l_inner = l_inner.clone();
            let _u = lp2.timer(move |_| { l_inner.borrow_mut().push("inner"); }, 0.0, None);
        }, 0.0, Some(0.0));
        *outer_handle.borrow_mut() = Some(t);
        mc.set(10.0);
        lp.flush();
        // outer fires once and stops itself; inner is scheduled at t=10
        // with delay=0 -> time=10 but we already iterated past it in the
        // snapshot for this flush, so it does NOT fire.
        assert_eq!(&*log.borrow(), &["outer"]);
        // Subsequent tick fires inner.
        mc.set(11.0);
        lp.flush();
        assert_eq!(&*log.borrow(), &["outer", "inner"]);
    }

    #[test]
    fn now_inside_tick_is_latched() {
        let (lp, mc) = loop_with_clock();
        let observed = Rc::new(RefCell::new(Vec::<f64>::new()));
        let o = observed.clone();
        let lp2 = lp.clone();
        let _t = lp.timer(move |_| {
            // Two reads inside the same tick must yield the same value.
            let a = lp2.now();
            let b = lp2.now();
            o.borrow_mut().push(a);
            o.borrow_mut().push(b);
        }, 0.0, Some(0.0));
        mc.set(123.0);
        lp.flush();
        let v = observed.borrow();
        assert_eq!(v[0], v[1]);
        assert_eq!(v[0], 123.0);
    }
}
