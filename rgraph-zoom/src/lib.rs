//! `rgraph-zoom` — Rust port of [d3-zoom](https://github.com/d3/d3-zoom).
//!
//! Provides:
//!
//! * [`Transform`](transform::Transform) — the affine zoom transform
//!   `(p) -> p*k + (x, y)`. Pure data + math, fully tested against d3
//!   fixtures.
//! * [`ZoomBehavior`](zoom::ZoomBehavior) — a pure pointer/wheel-driven
//!   gesture state machine that emits `start` / `zoom` / `end` events on
//!   a [`Dispatch`](rgraph_dispatch::Dispatch). Generic over a target
//!   key `K` and per-gesture datum `D`.
//! * Smooth animated zoom via integration with
//!   [`rgraph_transition::TransitionEngine`] +
//!   [`rgraph_interpolate::interpolate_zoom`] (van Wijk smooth-zoom path).
//!
//! # Why this port omits the DOM-attaching half
//!
//! d3-zoom in JavaScript attaches `wheel`, `mousedown`, `dblclick`,
//! `touchstart`/`touchmove`/`touchend`/`touchcancel` listeners directly
//! to selection nodes, and uses `dragDisable`/`dragEnable` to suppress
//! native text-selection during drag. In a Dioxus codebase the calling
//! component already wires `onwheel` / `onmousedown` / `onpointermove` /
//! `ondblclick` / `ontouchstart` (etc.) via `rsx!{}` and calls
//! `evt.prevent_default()` / `evt.stop_propagation()` on the Dioxus
//! event itself, so those layers have no idiomatic counterpart in Rust.
//!
//! This crate ports the **pure gesture math** — wheel-to-zoom,
//! mouse-pan, single- and two-finger touch (pinch + pan), double-click
//! zoom, click-distance suppression, and the full programmatic
//! `transform`/`scaleBy`/`scaleTo`/`translateBy`/`translateTo` API.
//!
//! # Quick example
//!
//! ```ignore
//! use rgraph_zoom::{ZoomBehavior, Transform, PointerInput, PointerId, WheelInput};
//! let z = ZoomBehavior::<u64, ()>::new();
//! z.scale_extent(0.25, 8.0);
//! z.scale_by(1, 2.0, Some([100.0, 100.0]), None);
//! assert_eq!(z.transform(&1).k, 2.0);
//!
//! // Wheel events from a Dioxus onwheel handler:
//! z.handle_wheel(
//!     1,
//!     WheelInput { delta_y: -120.0, delta_mode: 0, ctrl: false, x: 50.0, y: 50.0 },
//! );
//! ```

pub mod transform;
pub mod zoom;

// Convenience re-exports.
pub use transform::Transform;
pub use zoom::{
    DoubleClickInput, Extent, FilterContext, FilterSource, PointerId, PointerInput, WheelInput,
    ZoomBehavior, ZoomEvent, bridge_drag_to_zoom, default_constrain, default_filter,
    default_wheel_delta,
};
