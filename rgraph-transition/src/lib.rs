//! `rgraph-transition` — Rust port of d3-timer + d3-ease + d3-transition.
//!
//! This crate combines three d3 modules into one cohesive Rust API:
//!
//! * [`ease`] — easing functions (linear / quad / cubic / poly / sin / exp /
//!   circle / bounce / back / elastic), including parametric variants.
//! * [`timer`] — a pull-driven timer loop with a [`timer::Clock`] abstraction.
//!   Replaces d3-timer's `requestAnimationFrame` driver with explicit
//!   per-frame [`timer::TimerLoop::tick`] calls suitable for game / firmware
//!   / async event loops.
//! * [`transition`] — the d3-transition state machine, generic over a
//!   user-keyed target id `K` instead of DOM nodes. Backed by
//!   [`rgraph_dispatch`] for events.
//!
//! All three modules are independently usable. They are combined here both
//! for ergonomic single-import and because d3-transition depends on the
//! other two upstream.
//!
//! See module-level docs for usage details.

pub mod ease;
pub mod timer;
pub mod transition;

// Convenience re-exports of the most-used items.
pub use ease::EaseFn;
pub use timer::{Clock, InstantClock, ManualClock, Timer, TimerLoop};
pub use transition::{EventCtx, State, TransitionEngine, TransitionId};
