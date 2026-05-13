//! Port of `xyflow-core/src/utils/edge-toolbar.ts`.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use crate::types::edges::{AlignX, AlignY};
use crate::utils::edges::format::js_num;

#[inline]
fn align_x_to_percent(a: AlignX) -> f64 {
    match a {
        AlignX::Left => 0.0,
        AlignX::Center => 50.0,
        AlignX::Right => 100.0,
    }
}

#[inline]
fn align_y_to_percent(a: AlignY) -> f64 {
    match a {
        AlignY::Top => 0.0,
        AlignY::Center => 50.0,
        AlignY::Bottom => 100.0,
    }
}

/// Compute the CSS `transform` string for an edge toolbar.
///
/// The toolbar is anchored at `(x, y)` in flow-space and counter-scaled
/// by `1 / zoom` so it remains screen-pixel-sized regardless of the
/// current viewport zoom. Alignment offsets shift it relative to its
/// own bounding box.
///
/// Mirrors the TS `getEdgeToolbarTransform`. `align_x = Center`,
/// `align_y = Center` are the defaults, matching the JS signature.
#[must_use]
pub fn get_edge_toolbar_transform(x: f64, y: f64, zoom: f64, align_x: AlignX, align_y: AlignY) -> String {
    let inv_zoom = 1.0 / zoom;
    format!(
        "translate({}px, {}px) scale({}) translate({}%, {}%)",
        js_num(x),
        js_num(y),
        js_num(inv_zoom),
        js_num(-align_x_to_percent(align_x)),
        js_num(-align_y_to_percent(align_y))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_js() {
        // x=10, y=20, zoom=1 → scale(1), translate(-50%, -50%) by default.
        let s = get_edge_toolbar_transform(10.0, 20.0, 1.0, AlignX::Center, AlignY::Center);
        assert_eq!(s, "translate(10px, 20px) scale(1) translate(-50%, -50%)");
    }

    #[test]
    fn zoom_applies_inverse_scale() {
        // zoom=2 → scale(0.5)
        let s = get_edge_toolbar_transform(0.0, 0.0, 2.0, AlignX::Center, AlignY::Center);
        assert_eq!(s, "translate(0px, 0px) scale(0.5) translate(-50%, -50%)");
    }

    #[test]
    fn corner_alignment_combinations() {
        let s = get_edge_toolbar_transform(0.0, 0.0, 1.0, AlignX::Left, AlignY::Top);
        // -0% collapses to 0% via js_num (matches JS String(-0) = "0").
        assert_eq!(s, "translate(0px, 0px) scale(1) translate(0%, 0%)");
        let s = get_edge_toolbar_transform(0.0, 0.0, 1.0, AlignX::Right, AlignY::Bottom);
        assert_eq!(s, "translate(0px, 0px) scale(1) translate(-100%, -100%)");
    }
}
