//! Port of `xyflow-core/src/utils/dom.ts`.
//!
//! Status: implemented (phase 3).
//!
//! IMPORTANT — this is the most "DOMful" of the utility files
//! upstream. It contains helpers that read `getBoundingClientRect`,
//! search up the DOM tree, and inspect `composedPath` / `nodeName`.
//! None of those have idiomatic counterparts inside `rgraph-core`.
//!
//! The Rust port reimagines this file as **pure functions over
//! already-measured rectangles and pre-flattened metadata**. The
//! Dioxus consumer crate is responsible for actually performing the
//! DOM measurement (e.g. via `MountedData::get_client_rect()`) and for
//! collecting `nodeName` / `composedPath` info; everything below
//! takes `Rect`, `Dimensions`, and `&str` rather than DOM handles.

#![allow(clippy::module_name_repetitions)]

use crate::types::geometry::{Dimensions, Rect, Transform, XYPosition};
use crate::types::handles::{Handle, HandleType};
use crate::types::geometry::Position;
use crate::types::viewport::SnapGrid;
use crate::utils::general::{point_to_renderer_point, snap_position};

/// Pre-measured pointer / event view used in lieu of `MouseEvent` /
/// `TouchEvent`. The Dioxus consumer fills this in from its
/// `PointerEvent` / `TouchEvent` data.
///
/// Re-exported from [`crate::types::nodes::PointerEventLike`] — see
/// that module for the canonical definition.
pub use crate::types::nodes::PointerEventLike;

/// Mirrors the TS object returned by `getEventPosition`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventPosition {
    pub x: f64,
    pub y: f64,
}

/// Container bounds — replaces TS `containerBounds: DOMRect | null`.
///
/// Pre-measured by the consumer (e.g. via Dioxus
/// `MountedData::get_client_rect()`). We use [`Rect`] but only `x` /
/// `y` are read here; `width` / `height` are ignored.
pub type ContainerBounds = Option<Rect>;

/// Parameters for [`get_pointer_position`].
#[derive(Debug, Clone, Copy)]
pub struct GetPointerPositionParams {
    pub transform: Transform,
    pub snap_grid: SnapGrid,
    pub snap_to_grid: bool,
    pub container_bounds: ContainerBounds,
}

impl Default for GetPointerPositionParams {
    fn default() -> Self {
        Self {
            transform: Transform::IDENTITY,
            snap_grid: (0.0, 0.0),
            snap_to_grid: false,
            container_bounds: None,
        }
    }
}

/// Pointer position in flow coordinates plus its snapped form.
///
/// Mirrors the TS return of `getPointerPosition` (an `XYPosition` with
/// `xSnapped` / `ySnapped` mixed in).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerPosition {
    pub x: f64,
    pub y: f64,
    pub x_snapped: f64,
    pub y_snapped: f64,
}

/// Compute the flow-space pointer position for an event, with optional
/// grid snapping.
///
/// Mirrors the TS `getPointerPosition(event, { transform, snapGrid,
/// snapToGrid, containerBounds })`.
#[must_use]
pub fn get_pointer_position(
    event: &PointerEventLike,
    params: GetPointerPositionParams,
) -> PointerPosition {
    let raw = get_event_position(event, params.container_bounds);
    let pointer = point_to_renderer_point(
        XYPosition {
            x: raw.x,
            y: raw.y,
        },
        params.transform,
        false,
        (1.0, 1.0),
    );
    let snapped = if params.snap_to_grid {
        snap_position(pointer, params.snap_grid)
    } else {
        pointer
    };
    PointerPosition {
        x: pointer.x,
        y: pointer.y,
        x_snapped: snapped.x,
        y_snapped: snapped.y,
    }
}

/// Translate a [`PointerEventLike`] into container-space coordinates.
///
/// Mirrors `getEventPosition(event, bounds?)`.
#[must_use]
#[inline]
pub fn get_event_position(event: &PointerEventLike, bounds: ContainerBounds) -> EventPosition {
    EventPosition {
        x: event.client_x - bounds.map(|b| b.x).unwrap_or(0.0),
        y: event.client_y - bounds.map(|b| b.y).unwrap_or(0.0),
    }
}

/// Convenience constructor for [`Dimensions`] from raw `f64` width and
/// height — replaces TS `getDimensions(node)` which read
/// `node.offsetWidth` / `offsetHeight`. The Rust crate does not have a
/// DOM node here; the consumer measures and calls this helper purely
/// for readability.
#[must_use]
#[inline]
pub fn dimensions(width: f64, height: f64) -> Dimensions {
    Dimensions { width, height }
}

/// `nodeName`-based predicate for "is this an editable form element?".
///
/// Mirrors the TS `isInputDOMNode(event)` which inspects
/// `event.composedPath()[0].nodeName`. The Dioxus consumer must pass
/// the resolved `nodeName` (e.g. `"INPUT"`, `"TEXTAREA"`,
/// `"SELECT"`), the `contenteditable` attribute presence, and any
/// ancestor class names it has gathered.
#[must_use]
pub fn is_input_dom_node(
    node_name: &str,
    has_contenteditable: bool,
    ancestor_class_list_contains_nokey: bool,
) -> bool {
    let upper = node_name.to_ascii_uppercase();
    let is_input =
        matches!(upper.as_str(), "INPUT" | "SELECT" | "TEXTAREA") || has_contenteditable;
    is_input || ancestor_class_list_contains_nokey
}

/// `getHandleBounds` reimagined.
///
/// In TS this iterates DOM elements with class `.source` / `.target`,
/// reads each handle's `getBoundingClientRect` plus
/// `data-handleid` / `data-handlepos` attributes, and computes the
/// handle's flow-space rect.
///
/// In Rust the Dioxus consumer pre-collects an iterator of
/// [`HandleMeasurement`]s — one per handle DOM element — and we apply
/// the same coordinate transform here.
#[must_use]
pub fn build_handle_bounds(
    handle_type: HandleType,
    measurements: impl IntoIterator<Item = HandleMeasurement>,
    node_bounds_left: f64,
    node_bounds_top: f64,
    zoom: f64,
    node_id: &str,
) -> Vec<Handle> {
    measurements
        .into_iter()
        .map(|m| Handle {
            id: m.id,
            type_: handle_type,
            node_id: node_id.to_string(),
            position: m.position,
            x: (m.bounds_left - node_bounds_left) / zoom,
            y: (m.bounds_top - node_bounds_top) / zoom,
            width: m.width,
            height: m.height,
        })
        .collect()
}

/// Per-handle measurement passed into [`build_handle_bounds`].
///
/// `bounds_left` / `bounds_top` are the handle DOM element's
/// `getBoundingClientRect()` `.left` / `.top`, in viewport coordinates.
/// `width` / `height` are the rendered handle dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct HandleMeasurement {
    /// `data-handleid` value (or `None` if absent).
    pub id: Option<String>,
    /// `data-handlepos` value, parsed by the consumer.
    pub position: Position,
    pub bounds_left: f64,
    pub bounds_top: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_position_subtracts_container_bounds_and_unprojects() {
        let evt = PointerEventLike {
            client_x: 150.0,
            client_y: 200.0,
            ..Default::default()
        };
        let bounds = Some(Rect::new(50.0, 100.0, 0.0, 0.0));
        let p = get_pointer_position(
            &evt,
            GetPointerPositionParams {
                transform: Transform::new(0.0, 0.0, 2.0),
                container_bounds: bounds,
                ..Default::default()
            },
        );
        // raw = (150-50, 200-100) = (100, 100); unprojected with scale=2 = (50, 50)
        assert!((p.x - 50.0).abs() < 1e-9);
        assert!((p.y - 50.0).abs() < 1e-9);
        assert!((p.x_snapped - 50.0).abs() < 1e-9);
        assert!((p.y_snapped - 50.0).abs() < 1e-9);
    }

    #[test]
    fn pointer_position_snaps_when_requested() {
        let evt = PointerEventLike {
            client_x: 13.0,
            client_y: 7.0,
            ..Default::default()
        };
        let p = get_pointer_position(
            &evt,
            GetPointerPositionParams {
                transform: Transform::IDENTITY,
                snap_grid: (5.0, 5.0),
                snap_to_grid: true,
                container_bounds: None,
            },
        );
        assert_eq!(p.x, 13.0);
        assert_eq!(p.y, 7.0);
        // 13 -> 15, 7 -> 5 (round-half-to-even but here 13/5 = 2.6 → 3 → 15)
        assert_eq!(p.x_snapped, 15.0);
        assert_eq!(p.y_snapped, 5.0);
    }

    #[test]
    fn is_input_dom_node_classifies_inputs() {
        assert!(is_input_dom_node("INPUT", false, false));
        assert!(is_input_dom_node("input", false, false));
        assert!(is_input_dom_node("TEXTAREA", false, false));
        assert!(is_input_dom_node("SELECT", false, false));
        assert!(is_input_dom_node("DIV", true, false)); // contenteditable
        assert!(is_input_dom_node("DIV", false, true)); // .nokey ancestor
        assert!(!is_input_dom_node("DIV", false, false));
    }

    #[test]
    fn build_handle_bounds_translates_rects_to_flow_space() {
        let handles = vec![
            HandleMeasurement {
                id: Some("h1".into()),
                position: Position::Right,
                bounds_left: 200.0,
                bounds_top: 110.0,
                width: 8.0,
                height: 8.0,
            },
            HandleMeasurement {
                id: None,
                position: Position::Bottom,
                bounds_left: 150.0,
                bounds_top: 200.0,
                width: 8.0,
                height: 8.0,
            },
        ];
        let bounds = build_handle_bounds(HandleType::Source, handles, 100.0, 100.0, 2.0, "n1");
        assert_eq!(bounds.len(), 2);
        // (200-100)/2 = 50; (110-100)/2 = 5
        assert_eq!(bounds[0].x, 50.0);
        assert_eq!(bounds[0].y, 5.0);
        assert_eq!(bounds[0].id.as_deref(), Some("h1"));
        assert_eq!(bounds[0].type_, HandleType::Source);
        assert_eq!(bounds[0].node_id, "n1");
        // (150-100)/2 = 25; (200-100)/2 = 50
        assert_eq!(bounds[1].x, 25.0);
        assert_eq!(bounds[1].y, 50.0);
        assert!(bounds[1].id.is_none());
    }

    #[test]
    fn dimensions_helper() {
        let d = dimensions(10.0, 20.0);
        assert_eq!(d.width, 10.0);
        assert_eq!(d.height, 20.0);
    }
}
