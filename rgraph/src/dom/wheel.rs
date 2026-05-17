//! `dom::wheel` — Dioxus `WheelEvent` → `rgraph_zoom::WheelInput` adapter.
//!
//! Status: Phase 4 — implemented.
//!
//! ## What this module does
//!
//! Translates a Dioxus [`WheelData`] event (plus the `BoundingClientRect`
//! of the wrapping pane) into the [`WheelInput`] format consumed by
//! [`rgraph_zoom::ZoomBehavior`].
//!
//! ## `preventDefault` caveat
//!
//! Dioxus dispatches wheel events through React-style listeners that
//! cannot call `event.preventDefault()` from Rust (the listener runs
//! after the browser has already committed the scroll). The Phase 4
//! port mitigates this by:
//!
//! 1. Setting CSS `overscroll-behavior: contain` on the
//!    `.react-flow__renderer` element so vertical wheel events don't
//!    propagate to the document scroller.
//! 2. Forwarding the event to `rgraph_zoom` synchronously inside the
//!    handler, so any visible scroll-then-zoom flicker stays inside one
//!    frame.
//!
//! Native preventDefault remains a Phase 5 follow-up (an
//! `addEventListener('wheel', …, { passive: false })` shim mounted via
//! `use_eval`).

#![allow(clippy::module_name_repetitions)]

use dioxus::events::WheelData;
use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::point_interaction::{InteractionLocation, ModifiersInteraction};
use dioxus::prelude::Event;

use rgraph_zoom::WheelInput;

/// Convenience: a (browser-relative) bounding box used by the
/// converter to compute the wheel position in pane-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PaneBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Convert a Dioxus [`WheelData`] (plus the pane's bounding rect) into
/// the [`WheelInput`] consumed by [`rgraph_zoom::ZoomBehavior`].
///
/// The pane bounds are subtracted from the client-space pointer
/// coordinates so the resulting `(x, y)` are pane-local — that's what
/// d3-zoom's wheel handler expects.
///
/// `_modifiers` is read off the underlying event but only `ctrl_key`
/// is forwarded; the others are dropped (matching the d3-zoom source).
///
/// Note: [`WheelInput`] only carries `delta_y` because the zoom engine
/// is uni-axial. Horizontal wheel deltas are dropped — Phase 5 may
/// surface a separate `pan_on_scroll` channel if needed.
#[must_use]
pub fn from_dioxus(event: &Event<WheelData>, bounds: PaneBounds) -> WheelInput {
    let data: &WheelData = event;
    let (_delta_x, delta_y, delta_mode) = decompose_delta(data.delta());
    let client = <WheelData as InteractionLocation>::client_coordinates(data);
    let mods = <WheelData as ModifiersInteraction>::modifiers(data);
    WheelInput {
        delta_y,
        delta_mode,
        ctrl: mods.contains(Modifiers::CONTROL),
        x: client.x - bounds.x,
        y: client.y - bounds.y,
    }
}

fn decompose_delta(delta: WheelDelta) -> (f64, f64, u8) {
    match delta {
        WheelDelta::Pixels(v) => (v.x, v.y, 0),
        WheelDelta::Lines(v) => (v.x, v.y, 1),
        WheelDelta::Pages(v) => (v.x, v.y, 2),
    }
}

// `MouseButtonSet` import kept because future expansions may need to
// inspect button state in the wheel handler (e.g. middle-click pan).
#[allow(dead_code)]
type _MBS = ();

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::html::geometry::PixelsVector3D;

    #[test]
    fn decompose_delta_pixels_first() {
        let (dx, dy, dm) = decompose_delta(WheelDelta::Pixels(PixelsVector3D::new(1.0, 2.0, 0.0)));
        assert_eq!(dx, 1.0);
        assert_eq!(dy, 2.0);
        assert_eq!(dm, 0);
    }

    #[test]
    fn decompose_delta_lines_then_pages() {
        let (_, _, dm) = decompose_delta(WheelDelta::lines(0.0, 1.0, 0.0));
        assert_eq!(dm, 1);
        let (_, _, dm) = decompose_delta(WheelDelta::pages(0.0, 0.0, 1.0));
        assert_eq!(dm, 2);
    }
}
