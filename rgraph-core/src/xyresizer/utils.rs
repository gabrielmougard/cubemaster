//! Port of `xyflow-core/src/xyresizer/utils.ts`.
//!
//! Status: implemented (phase 7).
//!
//! Most of this file is the giant [`get_dimensions_after_resize`]
//! function. It is a direct line-by-line port of the TS source: it
//! computes the new `(x, y, width, height)` of a node being resized,
//! considering min/max boundaries, parent/child extents, and an
//! optional aspect ratio. The comment block on the TS function
//! explains the approach better than any rewrite would; refer to it
//! when reading the body below.

#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments)]

use crate::types::geometry::CoordinateExtent;
use crate::types::nodes::NodeOrigin;
use crate::utils::dom::PointerPosition;
use crate::xyresizer::types::{
    ControlDirection, ControlPosition, ResizeBoundaries, ResizeDirection, StartValues,
};

// ---------------------------------------------------------------------------
// get_resize_direction
// ---------------------------------------------------------------------------

/// Parameters for [`get_resize_direction`]. Mirrors the inline TS
/// `GetResizeDirectionParams`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GetResizeDirectionParams {
    pub width: f64,
    pub prev_width: f64,
    pub height: f64,
    pub prev_height: f64,
    pub affects_x: bool,
    pub affects_y: bool,
}

/// Compute the per-axis growth direction (`-1`, `0`, `+1`).
///
/// When the resize affects the `x` or `y` field of the node (i.e. the
/// user is dragging the left or top side), the direction is inverted
/// so that "grow" always points outwards from the node, regardless of
/// which control is being dragged.
#[must_use]
pub fn get_resize_direction(p: GetResizeDirectionParams) -> ResizeDirection {
    let delta_w = p.width - p.prev_width;
    let delta_h = p.height - p.prev_height;
    let mut x = if delta_w > 0.0 {
        1
    } else if delta_w < 0.0 {
        -1
    } else {
        0
    };
    let mut y = if delta_h > 0.0 {
        1
    } else if delta_h < 0.0 {
        -1
    } else {
        0
    };
    if delta_w != 0.0 && p.affects_x {
        x *= -1;
    }
    if delta_h != 0.0 && p.affects_y {
        y *= -1;
    }
    ResizeDirection { x, y }
}

// ---------------------------------------------------------------------------
// get_control_direction
// ---------------------------------------------------------------------------

/// Parse a [`ControlPosition`] into a [`ControlDirection`] mask.
#[must_use]
pub fn get_control_direction(control: ControlPosition) -> ControlDirection {
    ControlDirection {
        is_horizontal: control.is_horizontal(),
        is_vertical: control.is_vertical(),
        affects_x: control.affects_x(),
        affects_y: control.affects_y(),
    }
}

// ---------------------------------------------------------------------------
// Private clamp helpers
// ---------------------------------------------------------------------------

#[inline]
fn get_lower_extent_clamp(lower_extent: f64, lower_bound: f64) -> f64 {
    (lower_bound - lower_extent).max(0.0)
}

#[inline]
fn get_upper_extent_clamp(upper_extent: f64, upper_bound: f64) -> f64 {
    (upper_extent - upper_bound).max(0.0)
}

#[inline]
fn get_size_clamp(size: f64, min_size: f64, max_size: f64) -> f64 {
    (min_size - size).max(size - max_size).max(0.0)
}

#[inline]
fn xor(a: bool, b: bool) -> bool {
    a ^ b
}

// ---------------------------------------------------------------------------
// get_dimensions_after_resize
// ---------------------------------------------------------------------------

/// Result of [`get_dimensions_after_resize`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResizeResult {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
}

/// Compute the new dimensions and position of a node after a resize
/// tick.
///
/// Direct port of the TS `getDimensionsAfterResize`. See the upstream
/// JSDoc for the algorithm rationale; the body below is line-by-line
/// equivalent.
#[must_use]
pub fn get_dimensions_after_resize(
    start_values: StartValues,
    control_direction: ControlDirection,
    pointer_position: PointerPosition,
    boundaries: ResizeBoundaries,
    keep_aspect_ratio: bool,
    node_origin: NodeOrigin,
    extent: Option<CoordinateExtent>,
    child_extent: Option<CoordinateExtent>,
) -> ResizeResult {
    let mut affects_x = control_direction.affects_x;
    let mut affects_y = control_direction.affects_y;
    let is_horizontal = control_direction.is_horizontal;
    let is_vertical = control_direction.is_vertical;
    let is_diagonal = is_horizontal && is_vertical;

    let x_snapped = pointer_position.x_snapped;
    let y_snapped = pointer_position.y_snapped;
    let ResizeBoundaries {
        min_width,
        max_width,
        min_height,
        max_height,
    } = boundaries;

    let StartValues {
        x: start_x,
        y: start_y,
        width: start_width,
        height: start_height,
        aspect_ratio,
        ..
    } = start_values;

    let mut dist_x = if is_horizontal {
        (x_snapped - start_values.pointer_x).floor()
    } else {
        0.0
    };
    let mut dist_y = if is_vertical {
        (y_snapped - start_values.pointer_y).floor()
    } else {
        0.0
    };

    let new_width = start_width + if affects_x { -dist_x } else { dist_x };
    let new_height = start_height + if affects_y { -dist_y } else { dist_y };

    let origin_offset_x = -node_origin.0 * start_width;
    let origin_offset_y = -node_origin.1 * start_height;

    // Min / max clamp.
    let mut clamp_x = get_size_clamp(new_width, min_width, max_width);
    let mut clamp_y = get_size_clamp(new_height, min_height, max_height);

    // Parent extent clamp.
    if let Some(ext) = extent {
        let mut x_extent_clamp = 0.0;
        let mut y_extent_clamp = 0.0;
        if affects_x && dist_x < 0.0 {
            x_extent_clamp = get_lower_extent_clamp(start_x + dist_x + origin_offset_x, ext[0][0]);
        } else if !affects_x && dist_x > 0.0 {
            x_extent_clamp =
                get_upper_extent_clamp(start_x + new_width + origin_offset_x, ext[1][0]);
        }
        if affects_y && dist_y < 0.0 {
            y_extent_clamp = get_lower_extent_clamp(start_y + dist_y + origin_offset_y, ext[0][1]);
        } else if !affects_y && dist_y > 0.0 {
            y_extent_clamp =
                get_upper_extent_clamp(start_y + new_height + origin_offset_y, ext[1][1]);
        }
        clamp_x = clamp_x.max(x_extent_clamp);
        clamp_y = clamp_y.max(y_extent_clamp);
    }

    // Child extent clamp.
    if let Some(child_ext) = child_extent {
        let mut x_extent_clamp = 0.0;
        let mut y_extent_clamp = 0.0;
        if affects_x && dist_x > 0.0 {
            x_extent_clamp = get_upper_extent_clamp(start_x + dist_x, child_ext[0][0]);
        } else if !affects_x && dist_x < 0.0 {
            x_extent_clamp = get_lower_extent_clamp(start_x + new_width, child_ext[1][0]);
        }
        if affects_y && dist_y > 0.0 {
            y_extent_clamp = get_upper_extent_clamp(start_y + dist_y, child_ext[0][1]);
        } else if !affects_y && dist_y < 0.0 {
            y_extent_clamp = get_lower_extent_clamp(start_y + new_height, child_ext[1][1]);
        }
        clamp_x = clamp_x.max(x_extent_clamp);
        clamp_y = clamp_y.max(y_extent_clamp);
    }

    // Aspect-ratio clamps for the perpendicular axis.
    if keep_aspect_ratio {
        if is_horizontal {
            let aspect_height_clamp =
                get_size_clamp(new_width / aspect_ratio, min_height, max_height) * aspect_ratio;
            clamp_x = clamp_x.max(aspect_height_clamp);

            if let Some(ext) = extent {
                let aspect_extent_clamp = if (!affects_x && !affects_y)
                    || (affects_x && !affects_y && is_diagonal)
                {
                    get_upper_extent_clamp(
                        start_y + origin_offset_y + new_width / aspect_ratio,
                        ext[1][1],
                    ) * aspect_ratio
                } else {
                    get_lower_extent_clamp(
                        start_y
                            + origin_offset_y
                            + (if affects_x { dist_x } else { -dist_x }) / aspect_ratio,
                        ext[0][1],
                    ) * aspect_ratio
                };
                clamp_x = clamp_x.max(aspect_extent_clamp);
            }

            if let Some(child_ext) = child_extent {
                let aspect_extent_clamp = if (!affects_x && !affects_y)
                    || (affects_x && !affects_y && is_diagonal)
                {
                    get_lower_extent_clamp(start_y + new_width / aspect_ratio, child_ext[1][1])
                        * aspect_ratio
                } else {
                    get_upper_extent_clamp(
                        start_y + (if affects_x { dist_x } else { -dist_x }) / aspect_ratio,
                        child_ext[0][1],
                    ) * aspect_ratio
                };
                clamp_x = clamp_x.max(aspect_extent_clamp);
            }
        }

        if is_vertical {
            let aspect_width_clamp =
                get_size_clamp(new_height * aspect_ratio, min_width, max_width) / aspect_ratio;
            clamp_y = clamp_y.max(aspect_width_clamp);

            if let Some(ext) = extent {
                let aspect_extent_clamp = if (!affects_x && !affects_y)
                    || (affects_y && !affects_x && is_diagonal)
                {
                    get_upper_extent_clamp(
                        start_x + new_height * aspect_ratio + origin_offset_x,
                        ext[1][0],
                    ) / aspect_ratio
                } else {
                    get_lower_extent_clamp(
                        start_x
                            + (if affects_y { dist_y } else { -dist_y }) * aspect_ratio
                            + origin_offset_x,
                        ext[0][0],
                    ) / aspect_ratio
                };
                clamp_y = clamp_y.max(aspect_extent_clamp);
            }

            if let Some(child_ext) = child_extent {
                let aspect_extent_clamp = if (!affects_x && !affects_y)
                    || (affects_y && !affects_x && is_diagonal)
                {
                    get_lower_extent_clamp(start_x + new_height * aspect_ratio, child_ext[1][0])
                        / aspect_ratio
                } else {
                    get_upper_extent_clamp(
                        start_x + (if affects_y { dist_y } else { -dist_y }) * aspect_ratio,
                        child_ext[0][0],
                    ) / aspect_ratio
                };
                clamp_y = clamp_y.max(aspect_extent_clamp);
            }
        }
    }

    dist_y += if dist_y < 0.0 { clamp_y } else { -clamp_y };
    dist_x += if dist_x < 0.0 { clamp_x } else { -clamp_x };

    if keep_aspect_ratio {
        if is_diagonal {
            // Use the *clamped* new_width/new_height for the aspect
            // check (matches the TS, which uses `newWidth`/`newHeight`
            // computed *before* the clamps but with `aspectRatio`
            // applied).
            if new_width > new_height * aspect_ratio {
                dist_y = if xor(affects_x, affects_y) {
                    -dist_x
                } else {
                    dist_x
                } / aspect_ratio;
            } else {
                dist_x = if xor(affects_x, affects_y) {
                    -dist_y
                } else {
                    dist_y
                } * aspect_ratio;
            }
        } else if is_horizontal {
            dist_y = dist_x / aspect_ratio;
            affects_y = affects_x;
        } else {
            dist_x = dist_y * aspect_ratio;
            affects_x = affects_y;
        }
    }

    let x = if affects_x { start_x + dist_x } else { start_x };
    let y = if affects_y { start_y + dist_y } else { start_y };

    ResizeResult {
        width: start_width + if affects_x { -dist_x } else { dist_x },
        height: start_height + if affects_y { -dist_y } else { dist_y },
        x: node_origin.0 * dist_x * (if !affects_x { 1.0 } else { -1.0 }) + x,
        y: node_origin.1 * dist_y * (if !affects_y { 1.0 } else { -1.0 }) + y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pointer(x: f64, y: f64) -> PointerPosition {
        PointerPosition {
            x,
            y,
            x_snapped: x,
            y_snapped: y,
        }
    }

    #[test]
    fn control_position_directions() {
        let d = get_control_direction(ControlPosition::BottomRight);
        assert!(d.is_horizontal);
        assert!(d.is_vertical);
        assert!(!d.affects_x);
        assert!(!d.affects_y);

        let d = get_control_direction(ControlPosition::TopLeft);
        assert!(d.is_horizontal);
        assert!(d.is_vertical);
        assert!(d.affects_x);
        assert!(d.affects_y);

        let d = get_control_direction(ControlPosition::Right);
        assert!(d.is_horizontal);
        assert!(!d.is_vertical);

        let d = get_control_direction(ControlPosition::Top);
        assert!(!d.is_horizontal);
        assert!(d.is_vertical);
        assert!(d.affects_y);
    }

    #[test]
    fn resize_direction_signs() {
        let d = get_resize_direction(GetResizeDirectionParams {
            width: 50.0,
            prev_width: 40.0,
            height: 50.0,
            prev_height: 50.0,
            affects_x: false,
            affects_y: false,
        });
        assert_eq!(d, ResizeDirection { x: 1, y: 0 });

        let d = get_resize_direction(GetResizeDirectionParams {
            width: 30.0,
            prev_width: 40.0,
            height: 60.0,
            prev_height: 50.0,
            affects_x: false,
            affects_y: false,
        });
        assert_eq!(d, ResizeDirection { x: -1, y: 1 });
    }

    #[test]
    fn resize_direction_inverts_when_affecting_axis() {
        // Width grew (+1) but the control was on the left → user is
        // dragging the left side outwards. Direction inverts to -1.
        let d = get_resize_direction(GetResizeDirectionParams {
            width: 50.0,
            prev_width: 40.0,
            height: 50.0,
            prev_height: 50.0,
            affects_x: true,
            affects_y: false,
        });
        assert_eq!(d, ResizeDirection { x: -1, y: 0 });
    }

    #[test]
    fn dimensions_after_resize_bottom_right_grows_node() {
        // Drag the bottom-right control 10px right and 20px down.
        let start = StartValues {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            pointer_x: 100.0,
            pointer_y: 100.0,
            aspect_ratio: 1.0,
        };
        let result = get_dimensions_after_resize(
            start,
            get_control_direction(ControlPosition::BottomRight),
            pointer(110.0, 120.0),
            ResizeBoundaries::default(),
            false,
            (0.0, 0.0),
            None,
            None,
        );
        assert!((result.width - 110.0).abs() < 1e-9);
        assert!((result.height - 120.0).abs() < 1e-9);
        // x, y don't change for bottom-right
        assert!((result.x - 0.0).abs() < 1e-9);
        assert!((result.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn dimensions_after_resize_top_left_shifts_position() {
        // Drag the top-left control 10px right and 20px down → shrinks node.
        let start = StartValues {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            pointer_x: 0.0,
            pointer_y: 0.0,
            aspect_ratio: 1.0,
        };
        let result = get_dimensions_after_resize(
            start,
            get_control_direction(ControlPosition::TopLeft),
            pointer(10.0, 20.0),
            ResizeBoundaries::default(),
            false,
            (0.0, 0.0),
            None,
            None,
        );
        // Width = 100 - 10 = 90, height = 100 - 20 = 80.
        assert!((result.width - 90.0).abs() < 1e-9);
        assert!((result.height - 80.0).abs() < 1e-9);
        // Position shifts: x += 10, y += 20.
        assert!((result.x - 10.0).abs() < 1e-9);
        assert!((result.y - 20.0).abs() < 1e-9);
    }

    #[test]
    fn dimensions_after_resize_clamps_to_min_size() {
        // Drag bottom-right inward past min_width / min_height.
        let start = StartValues {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            pointer_x: 100.0,
            pointer_y: 100.0,
            aspect_ratio: 1.0,
        };
        let result = get_dimensions_after_resize(
            start,
            get_control_direction(ControlPosition::BottomRight),
            pointer(0.0, 0.0),
            ResizeBoundaries {
                min_width: 20.0,
                min_height: 20.0,
                max_width: f64::MAX,
                max_height: f64::MAX,
            },
            false,
            (0.0, 0.0),
            None,
            None,
        );
        assert!((result.width - 20.0).abs() < 1e-9);
        assert!((result.height - 20.0).abs() < 1e-9);
    }

    #[test]
    fn dimensions_after_resize_clamps_to_parent_extent() {
        // Parent extent forbids the node from growing past width 50.
        let start = StartValues {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 40.0,
            pointer_x: 40.0,
            pointer_y: 40.0,
            aspect_ratio: 1.0,
        };
        let result = get_dimensions_after_resize(
            start,
            get_control_direction(ControlPosition::BottomRight),
            pointer(200.0, 200.0),
            ResizeBoundaries::default(),
            false,
            (0.0, 0.0),
            Some([[0.0, 0.0], [50.0, 50.0]]),
            None,
        );
        assert!((result.width - 50.0).abs() < 1e-9);
        assert!((result.height - 50.0).abs() < 1e-9);
    }

    #[test]
    fn dimensions_after_resize_keeps_aspect_ratio_diagonal() {
        // Square node, drag the bottom-right control: width and height
        // should grow together.
        let start = StartValues {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            pointer_x: 100.0,
            pointer_y: 100.0,
            aspect_ratio: 1.0,
        };
        let result = get_dimensions_after_resize(
            start,
            get_control_direction(ControlPosition::BottomRight),
            pointer(150.0, 110.0),
            ResizeBoundaries::default(),
            true,
            (0.0, 0.0),
            None,
            None,
        );
        // The bigger movement (50 along x) wins; height should grow
        // to match width.
        assert!((result.width - result.height).abs() < 1e-9);
    }

    #[test]
    fn dimensions_after_resize_horizontal_only_with_aspect_ratio() {
        // Horizontal Right control, aspect-ratio mode → height
        // tracks width.
        let start = StartValues {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
            pointer_x: 100.0,
            pointer_y: 25.0,
            aspect_ratio: 2.0, // width/height
        };
        let result = get_dimensions_after_resize(
            start,
            get_control_direction(ControlPosition::Right),
            pointer(120.0, 25.0),
            ResizeBoundaries::default(),
            true,
            (0.0, 0.0),
            None,
            None,
        );
        // Width = 120, so height = 120/2 = 60.
        assert!((result.width - 120.0).abs() < 1e-9);
        assert!((result.height - 60.0).abs() < 1e-9);
    }
}
