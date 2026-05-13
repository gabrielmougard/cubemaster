//! Port of `xyflow-core/src/utils/edges/bezier-edge.ts`.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use crate::types::geometry::Position;
use crate::utils::edges::format::js_num;

/// Result of [`get_bezier_path`] / [`get_straight_path`] / [`get_smooth_step_path`].
///
/// Tuple shape: `(svg_path, label_x, label_y, offset_x, offset_y)`.
pub type EdgePathResult = (String, f64, f64, f64, f64);

/// Parameters for [`get_bezier_path`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GetBezierPathParams {
    pub source_x: f64,
    pub source_y: f64,
    pub source_position: Position,
    pub target_x: f64,
    pub target_y: f64,
    pub target_position: Position,
    pub curvature: f64,
}

impl Default for GetBezierPathParams {
    fn default() -> Self {
        // Same defaults as the TS getBezierPath signature.
        GetBezierPathParams {
            source_x: 0.0,
            source_y: 0.0,
            source_position: Position::Bottom,
            target_x: 0.0,
            target_y: 0.0,
            target_position: Position::Top,
            curvature: 0.25,
        }
    }
}

/// Compute the t=0.5 (cubic bezier) midpoint between source and target,
/// plus the absolute offsets from source.
///
/// Mirrors the TS `getBezierEdgeCenter`. Returns
/// `(centerX, centerY, offsetX, offsetY)`.
#[must_use]
pub fn get_bezier_edge_center(
    source_x: f64,
    source_y: f64,
    target_x: f64,
    target_y: f64,
    source_control_x: f64,
    source_control_y: f64,
    target_control_x: f64,
    target_control_y: f64,
) -> (f64, f64, f64, f64) {
    let center_x =
        source_x * 0.125 + source_control_x * 0.375 + target_control_x * 0.375 + target_x * 0.125;
    let center_y =
        source_y * 0.125 + source_control_y * 0.375 + target_control_y * 0.375 + target_y * 0.125;
    let offset_x = (center_x - source_x).abs();
    let offset_y = (center_y - source_y).abs();
    (center_x, center_y, offset_x, offset_y)
}

fn calculate_control_offset(distance: f64, curvature: f64) -> f64 {
    if distance >= 0.0 {
        0.5 * distance
    } else {
        curvature * 25.0 * (-distance).sqrt()
    }
}

fn get_control_with_curvature(
    pos: Position,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    c: f64,
) -> (f64, f64) {
    match pos {
        Position::Left => (x1 - calculate_control_offset(x1 - x2, c), y1),
        Position::Right => (x1 + calculate_control_offset(x2 - x1, c), y1),
        Position::Top => (x1, y1 - calculate_control_offset(y1 - y2, c)),
        Position::Bottom => (x1, y1 + calculate_control_offset(y2 - y1, c)),
    }
}

/// Compute everything needed to render a bezier edge between two
/// nodes.
///
/// Returns `(path, labelX, labelY, offsetX, offsetY)` where `path` is
/// directly usable as the `d` attribute of an SVG `<path>`.
///
/// Mirrors the TS `getBezierPath`.
#[must_use]
pub fn get_bezier_path(p: GetBezierPathParams) -> EdgePathResult {
    let (source_control_x, source_control_y) = get_control_with_curvature(
        p.source_position,
        p.source_x,
        p.source_y,
        p.target_x,
        p.target_y,
        p.curvature,
    );
    let (target_control_x, target_control_y) = get_control_with_curvature(
        p.target_position,
        p.target_x,
        p.target_y,
        p.source_x,
        p.source_y,
        p.curvature,
    );

    let (label_x, label_y, offset_x, offset_y) = get_bezier_edge_center(
        p.source_x,
        p.source_y,
        p.target_x,
        p.target_y,
        source_control_x,
        source_control_y,
        target_control_x,
        target_control_y,
    );

    let path = format!(
        "M{},{} C{},{} {},{} {},{}",
        js_num(p.source_x),
        js_num(p.source_y),
        js_num(source_control_x),
        js_num(source_control_y),
        js_num(target_control_x),
        js_num(target_control_y),
        js_num(p.target_x),
        js_num(p.target_y),
    );

    (path, label_x, label_y, offset_x, offset_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_path_from_doc_example() {
        // Doc example from getBezierPath JSDoc:
        //   source = { x: 0, y: 20 }, target = { x: 150, y: 100 }
        //   sourcePosition: Right, targetPosition: Left
        let (path, _lx, _ly, _ox, _oy) = get_bezier_path(GetBezierPathParams {
            source_x: 0.0,
            source_y: 20.0,
            source_position: Position::Right,
            target_x: 150.0,
            target_y: 100.0,
            target_position: Position::Left,
            curvature: 0.25,
        });
        // Source control: Right → x1 + 0.5*(x2-x1) = 0 + 75 = 75
        // Target control: Left  → x1 - 0.5*(x1-x2) = 150 - 75 = 75
        // y stays: source_control_y = 20, target_control_y = 100
        // path: "M0,20 C75,20 75,100 150,100"
        assert_eq!(path, "M0,20 C75,20 75,100 150,100");
    }

    #[test]
    fn bezier_label_centered() {
        let (_, lx, ly, ox, oy) = get_bezier_path(GetBezierPathParams {
            source_x: 0.0,
            source_y: 0.0,
            source_position: Position::Right,
            target_x: 100.0,
            target_y: 0.0,
            target_position: Position::Left,
            curvature: 0.25,
        });
        // sym → label is at (50, 0)
        assert!((lx - 50.0).abs() < 1e-9);
        assert!((ly - 0.0).abs() < 1e-9);
        assert!((ox - 50.0).abs() < 1e-9);
        assert!((oy - 0.0).abs() < 1e-9);
    }

    #[test]
    fn bezier_default_positions() {
        // Without setting positions, defaults are Bottom/Top.
        let p = GetBezierPathParams {
            source_x: 0.0,
            source_y: 0.0,
            target_x: 0.0,
            target_y: 100.0,
            ..Default::default()
        };
        let (path, _, _, _, _) = get_bezier_path(p);
        // Source control: Bottom → y1 + 0.5*(y2-y1) = 0 + 50 = 50
        // Target control: Top    → y1 - 0.5*(y1-y2) = 100 - (-50) = ...
        //   Actually: y1=100, y2=0, so y1 - 0.5*(y1-y2) = 100 - 50 = 50.
        assert_eq!(path, "M0,0 C0,50 0,50 0,100");
    }

    #[test]
    fn bezier_negative_distance_uses_sqrt_branch() {
        // Source on Right, but target is *left* of source → x1 - x2 > 0,
        // so the "Right" formula with distance = x2 - x1 is negative,
        // hitting the `curvature*25*sqrt(-d)` branch.
        let (path, _, _, _, _) = get_bezier_path(GetBezierPathParams {
            source_x: 100.0,
            source_y: 0.0,
            source_position: Position::Right,
            target_x: 0.0,
            target_y: 0.0,
            target_position: Position::Left,
            curvature: 0.25,
            // sourceY = targetY so calc is 1D.
        });
        // distance = x2 - x1 = -100 < 0 → control offset = 0.25 * 25 * sqrt(100) = 62.5
        // source control: x1 + 62.5 = 162.5
        // For target (Left): distance = x1 - x2 = 0 - 100 = -100 < 0
        //   target control: x1 - 62.5 = 0 - 62.5 = -62.5
        assert!(path.contains("162.5"));
        assert!(path.contains("-62.5"));
    }
}
