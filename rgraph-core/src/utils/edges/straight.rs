//! Port of `xyflow-core/src/utils/edges/straight-edge.ts`.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use crate::utils::edges::bezier::EdgePathResult;
use crate::utils::edges::format::js_num;
use crate::utils::edges::general::get_edge_center;

/// Parameters for [`get_straight_path`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GetStraightPathParams {
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
}

/// Straight-line path between source and target.
///
/// Mirrors the TS `getStraightPath`. Returns
/// `(path, labelX, labelY, offsetX, offsetY)`.
#[must_use]
pub fn get_straight_path(p: GetStraightPathParams) -> EdgePathResult {
    let (label_x, label_y, offset_x, offset_y) =
        get_edge_center(p.source_x, p.source_y, p.target_x, p.target_y);
    let path = format!(
        "M {},{}L {},{}",
        js_num(p.source_x),
        js_num(p.source_y),
        js_num(p.target_x),
        js_num(p.target_y),
    );
    (path, label_x, label_y, offset_x, offset_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_path_basic() {
        let (path, lx, ly, ox, oy) = get_straight_path(GetStraightPathParams {
            source_x: 0.0,
            source_y: 0.0,
            target_x: 100.0,
            target_y: 100.0,
        });
        assert_eq!(path, "M 0,0L 100,100");
        assert!((lx - 50.0).abs() < 1e-9);
        assert!((ly - 50.0).abs() < 1e-9);
        assert!((ox - 50.0).abs() < 1e-9);
        assert!((oy - 50.0).abs() < 1e-9);
    }

    #[test]
    fn straight_path_uses_js_number_format() {
        // Integer-valued floats should not have a trailing .0
        let (path, _, _, _, _) = get_straight_path(GetStraightPathParams {
            source_x: 1.0,
            source_y: 2.0,
            target_x: 3.5,
            target_y: 4.0,
        });
        assert_eq!(path, "M 1,2L 3.5,4");
    }
}
