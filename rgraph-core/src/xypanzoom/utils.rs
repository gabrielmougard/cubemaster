//! Port of `xyflow-core/src/xypanzoom/utils.ts`.
//!
//! Status: implemented (phase 4).

#![allow(clippy::module_name_repetitions)]

use rgraph_zoom::Transform as ZoomTransform;

use crate::types::geometry::Transform;
use crate::types::viewport::Viewport;

/// Convert a [`ZoomTransform`] (rgraph-zoom shape) into a [`Viewport`].
///
/// Mirrors the TS `transformToViewport`.
#[must_use]
#[inline]
pub fn transform_to_viewport(t: ZoomTransform) -> Viewport {
    Viewport {
        x: t.x,
        y: t.y,
        zoom: t.k,
    }
}

/// Convert a [`Viewport`] into a [`ZoomTransform`].
///
/// Mirrors the TS `viewportToTransform = zoomIdentity.translate(x, y).scale(zoom)`.
#[must_use]
#[inline]
pub fn viewport_to_transform(v: Viewport) -> ZoomTransform {
    ZoomTransform::IDENTITY.translate(v.x, v.y).scale(v.zoom)
}

/// Convert from `rgraph_zoom::Transform` to `crate::types::geometry::Transform`
/// (the `(tx, ty, scale)` tuple-struct used by the rest of `rgraph-core`).
#[must_use]
#[inline]
pub fn zoom_to_geometry_transform(t: ZoomTransform) -> Transform {
    Transform(t.x, t.y, t.k)
}

/// Inverse of [`zoom_to_geometry_transform`].
#[must_use]
#[inline]
pub fn geometry_to_zoom_transform(t: Transform) -> ZoomTransform {
    ZoomTransform::new(t.scale(), t.tx(), t.ty())
}

// ---------------------------------------------------------------------------
// Wheel-event delta computation
// ---------------------------------------------------------------------------

/// Wheel-event view, mirrors the fields the TS source reads from a
/// browser `WheelEvent`.
///
/// The Dioxus consumer fills this in from its `WheelEvent` data; we
/// keep a separate struct (rather than reusing
/// [`rgraph_zoom::WheelInput`]) because xyflow's `wheelDelta` requires
/// the `delta_x` axis and macOS detection too.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WheelEventLike {
    pub delta_x: f64,
    pub delta_y: f64,
    /// `WheelEvent.deltaMode`: 0 = pixels, 1 = lines, 2 = pages.
    pub delta_mode: u8,
    pub ctrl_key: bool,
    pub shift_key: bool,
}

/// Detect whether the host is macOS — only used by xyflow's
/// `wheelDelta` to apply a 10× factor on `ctrl + wheel`. The Rust
/// port can't sniff `navigator.userAgent` from the core crate, so the
/// caller passes the resolved boolean in.
#[must_use]
pub fn xy_wheel_delta(event: &WheelEventLike, is_mac_os: bool) -> f64 {
    let factor = if event.ctrl_key && is_mac_os { 10.0 } else { 1.0 };
    let unit = match event.delta_mode {
        1 => 0.05,
        m if m != 0 => 1.0,
        _ => 0.002,
    };
    -event.delta_y * unit * factor
}

// ---------------------------------------------------------------------------
// Right-click pan helper
// ---------------------------------------------------------------------------

/// `true` iff a right click should pan, based on the active
/// `pan_on_drag` config and the button that's down.
///
/// Mirrors `isRightClickPan(panOnDrag, usedButton)`.
#[must_use]
#[inline]
pub fn is_right_click_pan(
    pan_on_drag: &crate::types::panzoom::PanOnDrag,
    used_button: u8,
) -> bool {
    use crate::types::panzoom::PanOnDrag;
    used_button == 2 && matches!(pan_on_drag, PanOnDrag::Buttons(btns) if btns.contains(&2))
}

// ---------------------------------------------------------------------------
// Class-list "wrapped with class" helper
// ---------------------------------------------------------------------------

/// Returns `true` if any of the pre-collected `ancestor_classes`
/// matches `class_name`. Replaces the TS `event.target.closest('.foo')`
/// DOM walk; the caller pre-collects the class list of every ancestor.
#[must_use]
pub fn is_wrapped_with_class(ancestor_classes: &[String], class_name: &str) -> bool {
    if class_name.is_empty() {
        return false;
    }
    ancestor_classes.iter().any(|c| c == class_name)
}

// ---------------------------------------------------------------------------
// Default ease (cubic in-out)
// ---------------------------------------------------------------------------

/// Default cubic in-out easing, ported from d3-ease.
///
/// Used as the implicit `ease` for animated `set_viewport` /
/// `scale_to` / `scale_by` calls when the caller passes
/// `Option<EaseFn>::None`.
#[must_use]
pub fn default_ease(t: f64) -> f64 {
    let mut t = t * 2.0;
    if t <= 1.0 {
        return t * t * t / 2.0;
    }
    t -= 2.0;
    (t * t * t + 2.0) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::panzoom::PanOnDrag;

    #[test]
    fn viewport_transform_round_trip() {
        let v = Viewport::new(50.0, 100.0, 2.0);
        let t = viewport_to_transform(v);
        // Identity.translate(50,100).scale(2) → k=2, x=50, y=100.
        assert_eq!(t.k, 2.0);
        assert_eq!(t.x, 50.0);
        assert_eq!(t.y, 100.0);
        let back = transform_to_viewport(t);
        assert_eq!(back, v);
    }

    #[test]
    fn geometry_transform_round_trip() {
        let zt = ZoomTransform::new(1.5, 10.0, 20.0);
        let g = zoom_to_geometry_transform(zt);
        assert_eq!(g, Transform(10.0, 20.0, 1.5));
        assert_eq!(geometry_to_zoom_transform(g), zt);
    }

    #[test]
    fn wheel_delta_default_factor() {
        // Pixel mode (0), no ctrl: -delta * 0.002.
        let e = WheelEventLike {
            delta_y: -100.0,
            delta_mode: 0,
            ..Default::default()
        };
        assert!((xy_wheel_delta(&e, false) - 0.2).abs() < 1e-9);
    }

    #[test]
    fn wheel_delta_line_mode() {
        // Line mode (1): -delta * 0.05.
        let e = WheelEventLike {
            delta_y: 1.0,
            delta_mode: 1,
            ..Default::default()
        };
        assert!((xy_wheel_delta(&e, false) - (-0.05)).abs() < 1e-9);
    }

    #[test]
    fn wheel_delta_macos_ctrl_factor() {
        // ctrl on macOS gives a 10× boost.
        let e = WheelEventLike {
            delta_y: -1.0,
            delta_mode: 0,
            ctrl_key: true,
            ..Default::default()
        };
        let macos = xy_wheel_delta(&e, true);
        let other = xy_wheel_delta(&e, false);
        assert!((macos - 10.0 * other).abs() < 1e-9);
    }

    #[test]
    fn right_click_pan_only_when_button_listed() {
        assert!(!is_right_click_pan(&PanOnDrag::On, 2));
        assert!(!is_right_click_pan(&PanOnDrag::Off, 2));
        assert!(!is_right_click_pan(&PanOnDrag::Buttons(vec![0, 1]), 2));
        assert!(is_right_click_pan(&PanOnDrag::Buttons(vec![0, 2]), 2));
        // Wrong button → false even if listed.
        assert!(!is_right_click_pan(&PanOnDrag::Buttons(vec![0, 2]), 1));
    }

    #[test]
    fn is_wrapped_with_class_finds_match() {
        let ancestors = vec!["nodrag".to_string(), "react-flow__pane".to_string()];
        assert!(is_wrapped_with_class(&ancestors, "nodrag"));
        assert!(!is_wrapped_with_class(&ancestors, "missing"));
        assert!(!is_wrapped_with_class(&ancestors, ""));
    }

    #[test]
    fn default_ease_endpoints() {
        assert!((default_ease(0.0) - 0.0).abs() < 1e-9);
        assert!((default_ease(0.5) - 0.5).abs() < 1e-9);
        assert!((default_ease(1.0) - 1.0).abs() < 1e-9);
    }
}
