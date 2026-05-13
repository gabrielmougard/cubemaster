//! Port of `xyflow-core/src/utils/general.ts` — pure math.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use crate::types::geometry::{
    Box2d, CoordinateExtent, Dimensions, Rect, Transform, XYPosition,
};
use crate::types::nodes::{InternalNode, NodeExtent, NodeLike, NodeLookup, NodeOrigin};
use crate::types::viewport::{Padding, PaddingWithUnit, SnapGrid, Viewport};

// ---------------------------------------------------------------------------
// Numeric helpers
// ---------------------------------------------------------------------------

/// `clamp(val, min, max)` — clamps `val` into `[min, max]`.
///
/// Mirrors the TS `clamp(val, min = 0, max = 1)`. Rust has no default
/// arguments; for the common `[0, 1]` case use [`clamp01`].
#[must_use]
#[inline]
pub fn clamp(val: f64, min: f64, max: f64) -> f64 {
    val.max(min).min(max)
}

/// Convenience for `clamp(val, 0.0, 1.0)`.
#[must_use]
#[inline]
pub fn clamp01(val: f64) -> f64 {
    clamp(val, 0.0, 1.0)
}

/// `is_numeric(n)` — TS `isNumeric` returns `true` for non-NaN finite
/// numbers; we model the same predicate against an `f64`.
#[must_use]
#[inline]
pub fn is_numeric(n: f64) -> bool {
    n.is_finite()
}

// ---------------------------------------------------------------------------
// Position / extent clamping
// ---------------------------------------------------------------------------

/// Clamp a 2D position into the given coordinate extent, optionally
/// reserving space for an object's `dimensions` so the *bottom-right*
/// corner stays inside the extent too.
///
/// Mirrors the TS `clampPosition(position = {x:0,y:0}, extent, dimensions)`.
/// Pass [`Dimensions::ZERO`] when no inset is required.
#[must_use]
pub fn clamp_position(
    position: XYPosition,
    extent: CoordinateExtent,
    dimensions: Dimensions,
) -> XYPosition {
    XYPosition {
        x: clamp(position.x, extent[0][0], extent[1][0] - dimensions.width),
        y: clamp(position.y, extent[0][1], extent[1][1] - dimensions.height),
    }
}

/// Clamp a child node's position so it stays inside its parent's
/// rectangle.
///
/// Mirrors the TS `clampPositionToParent`.
#[must_use]
pub fn clamp_position_to_parent<D: Clone>(
    child_position: XYPosition,
    child_dimensions: Dimensions,
    parent: &InternalNode<D>,
) -> XYPosition {
    let parent_dim = get_node_dimensions(parent);
    let XYPosition {
        x: parent_x,
        y: parent_y,
    } = parent.internals.position_absolute;

    clamp_position(
        child_position,
        [
            [parent_x, parent_y],
            [parent_x + parent_dim.width, parent_y + parent_dim.height],
        ],
        child_dimensions,
    )
}

// ---------------------------------------------------------------------------
// Auto-pan helpers
// ---------------------------------------------------------------------------

/// One-axis auto-pan velocity calculator. Returns a value in `[-1, 1]`
/// that represents the share of `speed` to apply on this tick.
///
/// Mirrors the private TS `calcAutoPanVelocity`.
fn calc_auto_pan_velocity(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        clamp((value - min).abs(), 1.0, min) / min
    } else if value > max {
        -clamp((value - max).abs(), 1.0, min) / min
    } else {
        0.0
    }
}

/// Compute the per-tick auto-pan delta along the X and Y axes when
/// `pos` is near the edges of `bounds`.
///
/// Mirrors TS `calcAutoPan(pos, bounds, speed = 15, distance = 40)`.
#[must_use]
pub fn calc_auto_pan(pos: XYPosition, bounds: Dimensions, speed: f64, distance: f64) -> (f64, f64) {
    let x = calc_auto_pan_velocity(pos.x, distance, bounds.width - distance) * speed;
    let y = calc_auto_pan_velocity(pos.y, distance, bounds.height - distance) * speed;
    (x, y)
}

/// Convenience wrapper for [`calc_auto_pan`] using the JS defaults
/// (`speed = 15`, `distance = 40`).
#[must_use]
#[inline]
pub fn calc_auto_pan_default(pos: XYPosition, bounds: Dimensions) -> (f64, f64) {
    calc_auto_pan(pos, bounds, 15.0, 40.0)
}

// ---------------------------------------------------------------------------
// Box / rect arithmetic
// ---------------------------------------------------------------------------

/// Returns the bounding box of two boxes.
#[must_use]
#[inline]
pub fn get_bounds_of_boxes(b1: Box2d, b2: Box2d) -> Box2d {
    Box2d {
        x: b1.x.min(b2.x),
        y: b1.y.min(b2.y),
        x2: b1.x2.max(b2.x2),
        y2: b1.y2.max(b2.y2),
    }
}

/// Convert a [`Rect`] to a [`Box2d`].
#[must_use]
#[inline]
pub fn rect_to_box(r: Rect) -> Box2d {
    Box2d {
        x: r.x,
        y: r.y,
        x2: r.x + r.width,
        y2: r.y + r.height,
    }
}

/// Convert a [`Box2d`] back to a [`Rect`].
#[must_use]
#[inline]
pub fn box_to_rect(b: Box2d) -> Rect {
    Rect {
        x: b.x,
        y: b.y,
        width: b.x2 - b.x,
        height: b.y2 - b.y,
    }
}

/// Returns the bounding [`Rect`] of two rects.
#[must_use]
#[inline]
pub fn get_bounds_of_rects(r1: Rect, r2: Rect) -> Rect {
    box_to_rect(get_bounds_of_boxes(rect_to_box(r1), rect_to_box(r2)))
}

/// Returns the *overlapping area* (in square pixels) between two
/// rectangles. `0` means no overlap.
#[must_use]
pub fn get_overlapping_area(rect_a: Rect, rect_b: Rect) -> f64 {
    let x_overlap = (rect_a.x + rect_a.width)
        .min(rect_b.x + rect_b.width)
        .max(rect_a.x.max(rect_b.x))
        - rect_a.x.max(rect_b.x);
    let x_overlap = x_overlap.max(0.0);
    let y_overlap = (rect_a.y + rect_a.height)
        .min(rect_b.y + rect_b.height)
        .max(rect_a.y.max(rect_b.y))
        - rect_a.y.max(rect_b.y);
    let y_overlap = y_overlap.max(0.0);
    (x_overlap * y_overlap).ceil()
}

// ---------------------------------------------------------------------------
// Node geometry
// ---------------------------------------------------------------------------

/// Get the resolved width/height of any [`NodeLike`].
///
/// Falls back through `measured` → `width`/`height` → `initialWidth`/
/// `initialHeight`, matching the TS lookup order.
#[must_use]
pub fn get_node_dimensions<N: NodeLike + ?Sized>(node: &N) -> Dimensions {
    let measured = node.measured();
    let width = measured.and_then(|m| m.width)
        .or_else(|| node.raw_width())
        .or_else(|| node.initial_width())
        .unwrap_or(0.0);
    let height = measured.and_then(|m| m.height)
        .or_else(|| node.raw_height())
        .or_else(|| node.initial_height())
        .unwrap_or(0.0);
    Dimensions { width, height }
}

/// True iff the node has both a non-`None` resolved width and height.
#[must_use]
pub fn node_has_dimensions<N: NodeLike + ?Sized>(node: &N) -> bool {
    let measured = node.measured();
    let has_w = measured.and_then(|m| m.width).is_some()
        || node.raw_width().is_some()
        || node.initial_width().is_some();
    let has_h = measured.and_then(|m| m.height).is_some()
        || node.raw_height().is_some()
        || node.initial_height().is_some();
    has_w && has_h
}

/// Return the node's position with origin offset applied, in the same
/// coordinate system as `node.position`.
///
/// Mirrors the TS `getNodePositionWithOrigin(node, nodeOrigin)`.
#[must_use]
pub fn get_node_position_with_origin<N: NodeLike + ?Sized>(
    node: &N,
    fallback_origin: NodeOrigin,
) -> XYPosition {
    let dim = get_node_dimensions(node);
    let origin = node.origin().unwrap_or(fallback_origin);
    XYPosition {
        x: node.position().x - dim.width * origin.0,
        y: node.position().y - dim.height * origin.1,
    }
}

/// Return the rect occupied by `node` (in absolute coords for an
/// [`InternalNode`], or origin-adjusted user-space coords for a plain
/// [`Node`]).
///
/// Mirrors the TS `nodeToRect(node, nodeOrigin = [0, 0])`. To
/// disambiguate the two node flavours, we expose dedicated functions
/// rather than emulating the TS `isInternalNodeBase` type guard.
#[must_use]
pub fn user_node_to_rect<D: Clone>(node: &crate::types::nodes::Node<D>, node_origin: NodeOrigin) -> Rect {
    let dim = get_node_dimensions(node);
    let pos = get_node_position_with_origin(node, node_origin);
    Rect {
        x: pos.x,
        y: pos.y,
        width: dim.width,
        height: dim.height,
    }
}

/// [`InternalNode`] equivalent of [`user_node_to_rect`] that uses
/// `internals.positionAbsolute` rather than recomputing the origin.
#[must_use]
pub fn internal_node_to_rect<D: Clone>(node: &InternalNode<D>) -> Rect {
    let dim = get_node_dimensions(node);
    Rect {
        x: node.internals.position_absolute.x,
        y: node.internals.position_absolute.y,
        width: dim.width,
        height: dim.height,
    }
}

/// Convenience: produce a [`Box2d`] for a user node.
#[must_use]
#[inline]
pub fn user_node_to_box<D: Clone>(node: &crate::types::nodes::Node<D>, node_origin: NodeOrigin) -> Box2d {
    rect_to_box(user_node_to_rect(node, node_origin))
}

/// Convenience: produce a [`Box2d`] for an internal node.
#[must_use]
#[inline]
pub fn internal_node_to_box<D: Clone>(node: &InternalNode<D>) -> Box2d {
    rect_to_box(internal_node_to_rect(node))
}

// ---------------------------------------------------------------------------
// Snap / point ↔ renderer conversions
// ---------------------------------------------------------------------------

/// Snap a point to the nearest gridline.
///
/// `snap_grid = (1.0, 1.0)` is a no-op (matches the TS default).
#[must_use]
#[inline]
pub fn snap_position(position: XYPosition, snap_grid: SnapGrid) -> XYPosition {
    XYPosition {
        x: snap_grid.0 * (position.x / snap_grid.0).round(),
        y: snap_grid.1 * (position.y / snap_grid.1).round(),
    }
}

/// Convert a screen-space point to renderer (flow) coordinates.
///
/// `transform` is `(tx, ty, scale)`.
#[must_use]
pub fn point_to_renderer_point(
    point: XYPosition,
    transform: Transform,
    snap_to_grid: bool,
    snap_grid: SnapGrid,
) -> XYPosition {
    let Transform(tx, ty, scale) = transform;
    let pos = XYPosition {
        x: (point.x - tx) / scale,
        y: (point.y - ty) / scale,
    };
    if snap_to_grid {
        snap_position(pos, snap_grid)
    } else {
        pos
    }
}

/// Inverse of [`point_to_renderer_point`].
#[must_use]
#[inline]
pub fn renderer_point_to_point(point: XYPosition, transform: Transform) -> XYPosition {
    let Transform(tx, ty, scale) = transform;
    XYPosition {
        x: point.x * scale + tx,
        y: point.y * scale + ty,
    }
}

// ---------------------------------------------------------------------------
// Padding parsing & getViewportForBounds
// ---------------------------------------------------------------------------

/// Parsed pixel paddings used internally by `get_viewport_for_bounds`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ParsedPaddings {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
    pub x: f64,
    pub y: f64,
}

/// Parse a single padding value to a number of pixels.
fn parse_padding(padding: PaddingWithUnit, viewport: f64) -> f64 {
    match padding {
        PaddingWithUnit::Number(p) => ((viewport - viewport / (1.0 + p)) * 0.5).floor(),
        PaddingWithUnit::Px(p) => p.floor(),
        PaddingWithUnit::Percent(p) => (viewport * p * 0.01).floor(),
    }
}

/// Resolve a [`Padding`] into per-side pixel values.
fn parse_paddings(padding: Padding, width: f64, height: f64) -> ParsedPaddings {
    match padding {
        Padding::Zero => ParsedPaddings::default(),
        Padding::Single(p) => {
            let padding_x = parse_padding(p, width);
            let padding_y = parse_padding(p, height);
            ParsedPaddings {
                top: padding_y,
                right: padding_x,
                bottom: padding_y,
                left: padding_x,
                x: padding_x * 2.0,
                y: padding_y * 2.0,
            }
        }
        Padding::Sided {
            top,
            right,
            bottom,
            left,
            x,
            y,
        } => {
            let top = parse_padding(top.or(y).unwrap_or(PaddingWithUnit::Number(0.0)), height);
            let bottom = parse_padding(bottom.or(y).unwrap_or(PaddingWithUnit::Number(0.0)), height);
            let left = parse_padding(left.or(x).unwrap_or(PaddingWithUnit::Number(0.0)), width);
            let right = parse_padding(right.or(x).unwrap_or(PaddingWithUnit::Number(0.0)), width);
            ParsedPaddings {
                top,
                right,
                bottom,
                left,
                x: left + right,
                y: top + bottom,
            }
        }
    }
}

/// Calculate, for a viewport candidate, the actual paddings that would
/// be applied to `bounds`.
fn calculate_applied_paddings(
    bounds: Rect,
    x: f64,
    y: f64,
    zoom: f64,
    width: f64,
    height: f64,
) -> ParsedPaddings {
    let top_left = renderer_point_to_point(XYPosition::new(bounds.x, bounds.y), Transform(x, y, zoom));
    let bottom_right = renderer_point_to_point(
        XYPosition::new(bounds.x + bounds.width, bounds.y + bounds.height),
        Transform(x, y, zoom),
    );
    let right = width - bottom_right.x;
    let bottom = height - bottom_right.y;
    ParsedPaddings {
        left: top_left.x.floor(),
        top: top_left.y.floor(),
        right: right.floor(),
        bottom: bottom.floor(),
        x: 0.0,
        y: 0.0,
    }
}

/// Returns a viewport that encloses the given bounds with padding.
///
/// Mirrors `getViewportForBounds(bounds, width, height, minZoom, maxZoom, padding)`.
#[must_use]
pub fn get_viewport_for_bounds(
    bounds: Rect,
    width: f64,
    height: f64,
    min_zoom: f64,
    max_zoom: f64,
    padding: Padding,
) -> Viewport {
    let p = parse_paddings(padding, width, height);

    let x_zoom = (width - p.x) / bounds.width;
    let y_zoom = (height - p.y) / bounds.height;

    let zoom = x_zoom.min(y_zoom);
    let clamped_zoom = clamp(zoom, min_zoom, max_zoom);

    let bounds_center_x = bounds.x + bounds.width / 2.0;
    let bounds_center_y = bounds.y + bounds.height / 2.0;
    let x = width / 2.0 - bounds_center_x * clamped_zoom;
    let y = height / 2.0 - bounds_center_y * clamped_zoom;

    let new_padding = calculate_applied_paddings(bounds, x, y, clamped_zoom, width, height);

    let offset_left = (new_padding.left - p.left).min(0.0);
    let offset_top = (new_padding.top - p.top).min(0.0);
    let offset_right = (new_padding.right - p.right).min(0.0);
    let offset_bottom = (new_padding.bottom - p.bottom).min(0.0);

    Viewport {
        x: x - offset_left + offset_right,
        y: y - offset_top + offset_bottom,
        zoom: clamped_zoom,
    }
}

// ---------------------------------------------------------------------------
// NodeExtent helpers
// ---------------------------------------------------------------------------

/// Equivalent of TS `isCoordinateExtent(extent)` — returns `true` only
/// for the [`NodeExtent::Custom`] variant.
#[must_use]
#[inline]
pub fn is_coordinate_extent(extent: &NodeExtent) -> bool {
    matches!(extent, NodeExtent::Custom(_))
}

/// Convert a child position to absolute coordinates by walking up to
/// the parent.
///
/// Mirrors the TS `evaluateAbsolutePosition`.
#[must_use]
pub fn evaluate_absolute_position<D: Clone>(
    position: XYPosition,
    dimensions: Dimensions,
    parent_id: &str,
    node_lookup: &NodeLookup<D>,
    fallback_origin: NodeOrigin,
) -> XYPosition {
    let mut absolute = position;
    if let Some(parent) = node_lookup.get(parent_id) {
        let origin = parent.user.origin.unwrap_or(fallback_origin);
        absolute.x += parent.internals.position_absolute.x - dimensions.width * origin.0;
        absolute.y += parent.internals.position_absolute.y - dimensions.height * origin.1;
    }
    absolute
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

/// True iff the two sets contain exactly the same string elements.
///
/// `std::collections::HashSet<T>: PartialEq` already provides this, but
/// the function exists for parity with the TS export.
#[must_use]
pub fn are_sets_equal<T: Eq + std::hash::Hash>(
    a: &std::collections::HashSet<T>,
    b: &std::collections::HashSet<T>,
) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nodes::{InternalNode, Node};

    #[test]
    fn clamp_basics() {
        assert_eq!(clamp(0.5, 0.0, 1.0), 0.5);
        assert_eq!(clamp(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(clamp(2.0, 0.0, 1.0), 1.0);
        assert_eq!(clamp01(2.0), 1.0);
    }

    #[test]
    fn is_numeric_rejects_nan_and_infs() {
        assert!(is_numeric(0.0));
        assert!(is_numeric(-1.0));
        assert!(!is_numeric(f64::NAN));
        assert!(!is_numeric(f64::INFINITY));
        assert!(!is_numeric(f64::NEG_INFINITY));
    }

    #[test]
    fn clamp_position_with_dimensions() {
        let p = clamp_position(
            XYPosition::new(50.0, 50.0),
            [[0.0, 0.0], [100.0, 100.0]],
            Dimensions::new(20.0, 20.0),
        );
        assert_eq!(p, XYPosition::new(50.0, 50.0));
        let p2 = clamp_position(
            XYPosition::new(-10.0, -10.0),
            [[0.0, 0.0], [100.0, 100.0]],
            Dimensions::new(20.0, 20.0),
        );
        assert_eq!(p2, XYPosition::new(0.0, 0.0));
        // ensure dimensions reduce upper bound
        let p3 = clamp_position(
            XYPosition::new(200.0, 200.0),
            [[0.0, 0.0], [100.0, 100.0]],
            Dimensions::new(20.0, 20.0),
        );
        assert_eq!(p3, XYPosition::new(80.0, 80.0));
    }

    #[test]
    fn rect_box_round_trip() {
        let r = Rect::new(1.0, 2.0, 10.0, 20.0);
        let b = rect_to_box(r);
        assert_eq!(b, Box2d::new(1.0, 2.0, 11.0, 22.0));
        assert_eq!(box_to_rect(b), r);
    }

    #[test]
    fn get_bounds_of_rects_grows_to_envelope() {
        let r = get_bounds_of_rects(
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Rect::new(5.0, 5.0, 10.0, 10.0),
        );
        assert_eq!(r, Rect::new(0.0, 0.0, 15.0, 15.0));
    }

    #[test]
    fn get_overlapping_area_basic() {
        // Same rect → full overlap area.
        assert_eq!(
            get_overlapping_area(Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(0.0, 0.0, 10.0, 10.0)),
            100.0
        );
        // Half overlap.
        assert_eq!(
            get_overlapping_area(Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(5.0, 5.0, 10.0, 10.0)),
            25.0
        );
        // No overlap.
        assert_eq!(
            get_overlapping_area(Rect::new(0.0, 0.0, 10.0, 10.0), Rect::new(20.0, 20.0, 10.0, 10.0)),
            0.0
        );
    }

    #[test]
    fn point_renderer_round_trip() {
        let t = Transform(50.0, 100.0, 2.0);
        let screen = XYPosition::new(150.0, 200.0);
        let flow = point_to_renderer_point(screen, t, false, (1.0, 1.0));
        let screen_back = renderer_point_to_point(flow, t);
        assert!((screen.x - screen_back.x).abs() < 1e-9);
        assert!((screen.y - screen_back.y).abs() < 1e-9);
    }

    #[test]
    fn snap_position_rounds_to_grid() {
        assert_eq!(snap_position(XYPosition::new(7.0, 13.0), (5.0, 5.0)), XYPosition::new(5.0, 15.0));
        assert_eq!(snap_position(XYPosition::new(7.0, 13.0), (1.0, 1.0)), XYPosition::new(7.0, 13.0));
    }

    #[test]
    fn get_node_dimensions_falls_back() {
        let mut n: Node<()> = Node::minimal("n1", 0.0, 0.0);
        assert_eq!(get_node_dimensions(&n), Dimensions::ZERO);
        n.initial_width = Some(50.0);
        n.initial_height = Some(60.0);
        assert_eq!(get_node_dimensions(&n), Dimensions::new(50.0, 60.0));
        n.width = Some(100.0);
        assert_eq!(get_node_dimensions(&n).width, 100.0);
        n.measured = Some(crate::types::nodes::MeasuredDimensions {
            width: Some(200.0),
            height: None,
        });
        assert_eq!(get_node_dimensions(&n).width, 200.0);
        assert_eq!(get_node_dimensions(&n).height, 60.0);
    }

    #[test]
    fn node_has_dimensions_handles_partial() {
        let mut n: Node<()> = Node::minimal("n1", 0.0, 0.0);
        assert!(!node_has_dimensions(&n));
        n.initial_width = Some(10.0);
        assert!(!node_has_dimensions(&n));
        n.initial_height = Some(10.0);
        assert!(node_has_dimensions(&n));
    }

    #[test]
    fn get_viewport_for_bounds_centers_no_padding() {
        let v = get_viewport_for_bounds(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            1000.0,
            1000.0,
            0.5,
            2.0,
            Padding::Zero,
        );
        // The bounds are 100x100, viewport 1000x1000, so zoom should be 10
        // but clamped by max_zoom = 2. Center the bounds in the viewport.
        assert!((v.zoom - 2.0).abs() < 1e-9);
        // 1000/2 - 50*2 = 500 - 100 = 400.
        assert!((v.x - 400.0).abs() < 1e-9);
        assert!((v.y - 400.0).abs() < 1e-9);
    }

    #[test]
    fn evaluate_absolute_position_offsets_by_parent() {
        let mut parent: InternalNode<()> = InternalNode::from_user(Node::minimal("p", 100.0, 200.0));
        parent.internals.position_absolute = XYPosition::new(100.0, 200.0);
        let mut lookup: NodeLookup<()> = NodeLookup::default();
        lookup.insert("p".into(), parent);
        let abs = evaluate_absolute_position(
            XYPosition::new(10.0, 20.0),
            Dimensions::ZERO,
            "p",
            &lookup,
            (0.0, 0.0),
        );
        assert_eq!(abs, XYPosition::new(110.0, 220.0));
    }

    #[test]
    fn parse_padding_units() {
        assert_eq!(parse_padding(PaddingWithUnit::Px(40.0), 1000.0), 40.0);
        assert_eq!(parse_padding(PaddingWithUnit::Percent(10.0), 1000.0), 100.0);
        // Number means "shrink so 1+p factor fits"
        let v = parse_padding(PaddingWithUnit::Number(0.1), 1000.0);
        // (1000 - 1000/1.1)*0.5 ≈ (1000 - 909.0909)*0.5 ≈ 45.45 → floor → 45
        assert_eq!(v, 45.0);
    }
}
