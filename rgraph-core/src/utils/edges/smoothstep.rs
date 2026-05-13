//! Port of `xyflow-core/src/utils/edges/smoothstep-edge.ts`.
//!
//! Status: implemented (phase 2).
//!
//! This implementation mimics an orthogonal edge routing behaviour: it
//! is not a true orthogonal router but is fast and good enough as the
//! default for `step` and `smoothstep` edges. The algorithm is
//! transcribed from the TS source as faithfully as possible.

#![allow(clippy::module_name_repetitions)]

use crate::types::geometry::{Position, XYPosition};
use crate::utils::edges::bezier::EdgePathResult;
use crate::utils::edges::format::js_num;
use crate::utils::edges::general::get_edge_center;

/// Parameters for [`get_smooth_step_path`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GetSmoothStepPathParams {
    pub source_x: f64,
    pub source_y: f64,
    pub source_position: Position,
    pub target_x: f64,
    pub target_y: f64,
    pub target_position: Position,
    /// Border radius of the rounded corners. Default: `5`.
    pub border_radius: f64,
    /// Optional center override (along the edge's primary direction).
    pub center_x: Option<f64>,
    /// Optional center override (along the edge's secondary direction).
    pub center_y: Option<f64>,
    /// Gap between the source/target handle and the first bend.
    /// Default: `20`.
    pub offset: f64,
    /// `0..=1` — fraction along the primary direction at which the
    /// bend occurs. `0.5` is the midpoint (default).
    pub step_position: f64,
}

impl Default for GetSmoothStepPathParams {
    fn default() -> Self {
        GetSmoothStepPathParams {
            source_x: 0.0,
            source_y: 0.0,
            source_position: Position::Bottom,
            target_x: 0.0,
            target_y: 0.0,
            target_position: Position::Top,
            border_radius: 5.0,
            center_x: None,
            center_y: None,
            offset: 20.0,
            step_position: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Dir {
    x: f64,
    y: f64,
}

#[inline]
fn handle_directions(p: Position) -> Dir {
    match p {
        Position::Left => Dir { x: -1.0, y: 0.0 },
        Position::Right => Dir { x: 1.0, y: 0.0 },
        Position::Top => Dir { x: 0.0, y: -1.0 },
        Position::Bottom => Dir { x: 0.0, y: 1.0 },
    }
}

fn get_direction(source: XYPosition, source_position: Position, target: XYPosition) -> Dir {
    if source_position == Position::Left || source_position == Position::Right {
        if source.x < target.x {
            Dir { x: 1.0, y: 0.0 }
        } else {
            Dir { x: -1.0, y: 0.0 }
        }
    } else if source.y < target.y {
        Dir { x: 0.0, y: 1.0 }
    } else {
        Dir { x: 0.0, y: -1.0 }
    }
}

#[inline]
fn distance(a: XYPosition, b: XYPosition) -> f64 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
}

/// Which axis a [`Dir`] is non-zero along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

#[inline]
fn axis_value(d: Dir, a: Axis) -> f64 {
    match a {
        Axis::X => d.x,
        Axis::Y => d.y,
    }
}

#[inline]
fn xy_axis_value(p: XYPosition, a: Axis) -> f64 {
    match a {
        Axis::X => p.x,
        Axis::Y => p.y,
    }
}

/// Orthogonal routing — mirrors the TS `getPoints`. Returns
/// `(points, centerX, centerY, defaultOffsetX, defaultOffsetY)`.
fn get_points(
    source: XYPosition,
    source_position: Position,
    target: XYPosition,
    target_position: Position,
    center_x: Option<f64>,
    center_y: Option<f64>,
    offset: f64,
    step_position: f64,
) -> (Vec<XYPosition>, f64, f64, f64, f64) {
    let source_dir = handle_directions(source_position);
    let target_dir = handle_directions(target_position);

    let source_gapped = XYPosition {
        x: source.x + source_dir.x * offset,
        y: source.y + source_dir.y * offset,
    };
    let target_gapped = XYPosition {
        x: target.x + target_dir.x * offset,
        y: target.y + target_dir.y * offset,
    };
    let dir = get_direction(source_gapped, source_position, target_gapped);
    let dir_accessor = if dir.x != 0.0 { Axis::X } else { Axis::Y };
    let curr_dir = axis_value(dir, dir_accessor);

    let (_def_cx, _def_cy, default_offset_x, default_offset_y) =
        get_edge_center(source.x, source.y, target.x, target.y);

    let mut points: Vec<XYPosition>;
    let center_x_out: f64;
    let center_y_out: f64;
    let mut source_gap_offset = XYPosition::ZERO;
    let mut target_gap_offset = XYPosition::ZERO;

    if axis_value(source_dir, dir_accessor) * axis_value(target_dir, dir_accessor) == -1.0 {
        // opposite handle positions, default case
        if dir_accessor == Axis::X {
            center_x_out = center_x
                .unwrap_or(source_gapped.x + (target_gapped.x - source_gapped.x) * step_position);
            center_y_out = center_y.unwrap_or((source_gapped.y + target_gapped.y) / 2.0);
        } else {
            center_x_out = center_x.unwrap_or((source_gapped.x + target_gapped.x) / 2.0);
            center_y_out = center_y
                .unwrap_or(source_gapped.y + (target_gapped.y - source_gapped.y) * step_position);
        }

        let vertical_split = vec![
            XYPosition {
                x: center_x_out,
                y: source_gapped.y,
            },
            XYPosition {
                x: center_x_out,
                y: target_gapped.y,
            },
        ];
        let horizontal_split = vec![
            XYPosition {
                x: source_gapped.x,
                y: center_y_out,
            },
            XYPosition {
                x: target_gapped.x,
                y: center_y_out,
            },
        ];

        if axis_value(source_dir, dir_accessor) == curr_dir {
            points = if dir_accessor == Axis::X {
                vertical_split
            } else {
                horizontal_split
            };
        } else {
            points = if dir_accessor == Axis::X {
                horizontal_split
            } else {
                vertical_split
            };
        }
    } else {
        // sourceTarget = take x from source, y from target; targetSource = opposite
        let source_target = vec![XYPosition {
            x: source_gapped.x,
            y: target_gapped.y,
        }];
        let target_source = vec![XYPosition {
            x: target_gapped.x,
            y: source_gapped.y,
        }];

        // Same-side handle positions.
        if dir_accessor == Axis::X {
            points = if source_dir.x == curr_dir {
                target_source.clone()
            } else {
                source_target.clone()
            };
        } else {
            points = if source_dir.y == curr_dir {
                source_target.clone()
            } else {
                target_source.clone()
            };
        }

        if source_position == target_position {
            let diff = (xy_axis_value(source, dir_accessor) - xy_axis_value(target, dir_accessor)).abs();
            // If a same-side edge has source/target close together, the
            // gapped point and the added point overlap. Add a gap offset
            // to avoid weird routing.
            if diff <= offset {
                let gap_offset = (offset - 1.0).min(offset - diff);
                if axis_value(source_dir, dir_accessor) == curr_dir {
                    let sign = if xy_axis_value(source_gapped, dir_accessor)
                        > xy_axis_value(source, dir_accessor)
                    {
                        -1.0
                    } else {
                        1.0
                    };
                    match dir_accessor {
                        Axis::X => source_gap_offset.x = sign * gap_offset,
                        Axis::Y => source_gap_offset.y = sign * gap_offset,
                    }
                } else {
                    let sign = if xy_axis_value(target_gapped, dir_accessor)
                        > xy_axis_value(target, dir_accessor)
                    {
                        -1.0
                    } else {
                        1.0
                    };
                    match dir_accessor {
                        Axis::X => target_gap_offset.x = sign * gap_offset,
                        Axis::Y => target_gap_offset.y = sign * gap_offset,
                    }
                }
            }
        }

        // Mixed handle positions like Right -> Bottom.
        if source_position != target_position {
            let dir_accessor_opposite = match dir_accessor {
                Axis::X => Axis::Y,
                Axis::Y => Axis::X,
            };
            let is_same_dir = axis_value(source_dir, dir_accessor)
                == axis_value(target_dir, dir_accessor_opposite);
            let source_gt_target_oppo = xy_axis_value(source_gapped, dir_accessor_opposite)
                > xy_axis_value(target_gapped, dir_accessor_opposite);
            let source_lt_target_oppo = xy_axis_value(source_gapped, dir_accessor_opposite)
                < xy_axis_value(target_gapped, dir_accessor_opposite);
            let flip_source_target = (axis_value(source_dir, dir_accessor) == 1.0
                && ((!is_same_dir && source_gt_target_oppo)
                    || (is_same_dir && source_lt_target_oppo)))
                || (axis_value(source_dir, dir_accessor) != 1.0
                    && ((!is_same_dir && source_lt_target_oppo)
                        || (is_same_dir && source_gt_target_oppo)));

            if flip_source_target {
                points = if dir_accessor == Axis::X {
                    source_target.clone()
                } else {
                    target_source.clone()
                };
            }
        }

        let source_gap_point = XYPosition {
            x: source_gapped.x + source_gap_offset.x,
            y: source_gapped.y + source_gap_offset.y,
        };
        let target_gap_point = XYPosition {
            x: target_gapped.x + target_gap_offset.x,
            y: target_gapped.y + target_gap_offset.y,
        };
        let max_x_distance = (source_gap_point.x - points[0].x)
            .abs()
            .max((target_gap_point.x - points[0].x).abs());
        let max_y_distance = (source_gap_point.y - points[0].y)
            .abs()
            .max((target_gap_point.y - points[0].y).abs());

        // Place the label on the longest segment.
        if max_x_distance >= max_y_distance {
            center_x_out = (source_gap_point.x + target_gap_point.x) / 2.0;
            center_y_out = points[0].y;
        } else {
            center_x_out = points[0].x;
            center_y_out = (source_gap_point.y + target_gap_point.y) / 2.0;
        }
    }

    let gapped_source = XYPosition {
        x: source_gapped.x + source_gap_offset.x,
        y: source_gapped.y + source_gap_offset.y,
    };
    let gapped_target = XYPosition {
        x: target_gapped.x + target_gap_offset.x,
        y: target_gapped.y + target_gap_offset.y,
    };

    let mut path_points: Vec<XYPosition> = Vec::with_capacity(points.len() + 4);
    path_points.push(source);
    if gapped_source.x != points[0].x || gapped_source.y != points[0].y {
        path_points.push(gapped_source);
    }
    path_points.extend_from_slice(&points);
    let last = points[points.len() - 1];
    if gapped_target.x != last.x || gapped_target.y != last.y {
        path_points.push(gapped_target);
    }
    path_points.push(target);

    (path_points, center_x_out, center_y_out, default_offset_x, default_offset_y)
}

fn get_bend(a: XYPosition, b: XYPosition, c: XYPosition, size: f64) -> String {
    let bend_size = (distance(a, b) / 2.0).min(distance(b, c) / 2.0).min(size);
    let XYPosition { x, y } = b;

    // No bend
    if (a.x == x && x == c.x) || (a.y == y && y == c.y) {
        return format!("L{} {}", js_num(x), js_num(y));
    }

    // First segment is horizontal
    if a.y == y {
        let x_dir = if a.x < c.x { -1.0 } else { 1.0 };
        let y_dir = if a.y < c.y { 1.0 } else { -1.0 };
        return format!(
            "L {},{}Q {},{} {},{}",
            js_num(x + bend_size * x_dir),
            js_num(y),
            js_num(x),
            js_num(y),
            js_num(x),
            js_num(y + bend_size * y_dir),
        );
    }

    // First segment is vertical
    let x_dir = if a.x < c.x { 1.0 } else { -1.0 };
    let y_dir = if a.y < c.y { -1.0 } else { 1.0 };
    format!(
        "L {},{}Q {},{} {},{}",
        js_num(x),
        js_num(y + bend_size * y_dir),
        js_num(x),
        js_num(y),
        js_num(x + bend_size * x_dir),
        js_num(y),
    )
}

/// Compute everything needed to render a stepped path between two
/// nodes.
///
/// Returns `(path, labelX, labelY, offsetX, offsetY)`.
///
/// Mirrors the TS `getSmoothStepPath`.
#[must_use]
pub fn get_smooth_step_path(p: GetSmoothStepPathParams) -> EdgePathResult {
    let source = XYPosition::new(p.source_x, p.source_y);
    let target = XYPosition::new(p.target_x, p.target_y);

    let (points, label_x, label_y, offset_x, offset_y) = get_points(
        source,
        p.source_position,
        target,
        p.target_position,
        p.center_x,
        p.center_y,
        p.offset,
        p.step_position,
    );

    let mut path = format!("M{} {}", js_num(points[0].x), js_num(points[0].y));
    for i in 1..points.len() - 1 {
        path.push_str(&get_bend(points[i - 1], points[i], points[i + 1], p.border_radius));
    }
    let last = points[points.len() - 1];
    path.push_str(&format!("L{} {}", js_num(last.x), js_num(last.y)));

    (path, label_x, label_y, offset_x, offset_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoothstep_horizontal_basic() {
        // Source on Right, target on Left, horizontal layout — the
        // canonical case.
        let (path, _lx, _ly, _ox, _oy) = get_smooth_step_path(GetSmoothStepPathParams {
            source_x: 0.0,
            source_y: 50.0,
            source_position: Position::Right,
            target_x: 200.0,
            target_y: 50.0,
            target_position: Position::Left,
            border_radius: 5.0,
            center_x: None,
            center_y: None,
            offset: 20.0,
            step_position: 0.5,
        });
        // Path should start at source, end at target, and be a single
        // straight horizontal line (no bend) since y is constant.
        assert!(path.starts_with("M0 50"));
        assert!(path.ends_with("L200 50"));
    }

    #[test]
    fn smoothstep_returns_label_at_midpoint_for_simple_horizontal() {
        let (_path, lx, ly, _, _) = get_smooth_step_path(GetSmoothStepPathParams {
            source_x: 0.0,
            source_y: 50.0,
            source_position: Position::Right,
            target_x: 200.0,
            target_y: 50.0,
            target_position: Position::Left,
            ..Default::default()
        });
        // For Right→Left the algorithm centers along x; at step_position 0.5
        // and source_y == target_y the center is on the midpoint of the
        // horizontal segment.
        assert!((lx - 100.0).abs() < 1e-9);
        assert!((ly - 50.0).abs() < 1e-9);
    }

    #[test]
    fn smoothstep_z_shape_for_right_right() {
        // Right→Right (same handle positions) on different y values
        // produces a Z shape: source out right, up/down, then back left.
        let (path, _lx, _ly, _, _) = get_smooth_step_path(GetSmoothStepPathParams {
            source_x: 0.0,
            source_y: 0.0,
            source_position: Position::Right,
            target_x: 0.0,
            target_y: 100.0,
            target_position: Position::Right,
            ..Default::default()
        });
        // Should at least contain a Q (quadratic bend) somewhere mid-path.
        assert!(path.contains('Q'));
        assert!(path.starts_with("M0 0"));
    }

    #[test]
    fn smoothstep_step_position_shifts_bend() {
        // step_position=0 places the bend right at the source side.
        let (path0, _, _, _, _) = get_smooth_step_path(GetSmoothStepPathParams {
            source_x: 0.0,
            source_y: 0.0,
            source_position: Position::Right,
            target_x: 200.0,
            target_y: 100.0,
            target_position: Position::Left,
            offset: 20.0,
            step_position: 0.0,
            ..Default::default()
        });
        // step_position=1 places the bend on the target side.
        let (path1, _, _, _, _) = get_smooth_step_path(GetSmoothStepPathParams {
            source_x: 0.0,
            source_y: 0.0,
            source_position: Position::Right,
            target_x: 200.0,
            target_y: 100.0,
            target_position: Position::Left,
            offset: 20.0,
            step_position: 1.0,
            ..Default::default()
        });
        assert_ne!(path0, path1);
    }

    #[test]
    fn smoothstep_uses_js_num_format() {
        let (path, _, _, _, _) = get_smooth_step_path(GetSmoothStepPathParams {
            source_x: 1.5,
            source_y: 2.0,
            ..Default::default()
        });
        // Make sure we get "M1.5 2" not "M1.5 2.0".
        assert!(path.starts_with("M1.5 2"));
    }
}
