//! Port of `xyflow-core/src/utils/node-toolbar.ts`.
//!
//! Status: implemented (phase 2).

#![allow(clippy::module_name_repetitions)]

use crate::types::geometry::{Position, Rect};
use crate::types::nodes::Align;
use crate::types::viewport::Viewport;
use crate::utils::edges::format::js_num;

/// Compute the CSS `transform` string that positions a node toolbar
/// relative to a node, taking the current viewport zoom/pan into
/// account.
///
/// Mirrors the TS `getNodeToolbarTransform`. Returns a string of the
/// form `"translate(<X>px, <Y>px) translate(<sx>%, <sy>%)"` ready to
/// be assigned to a `style.transform` value.
#[must_use]
pub fn get_node_toolbar_transform(
    node_rect: Rect,
    viewport: Viewport,
    position: Position,
    offset: f64,
    align: Align,
) -> String {
    let alignment_offset: f64 = match align {
        Align::Start => 0.0,
        Align::Center => 0.5,
        Align::End => 1.0,
    };

    // Default = Position::Top
    let mut pos_x = (node_rect.x + node_rect.width * alignment_offset) * viewport.zoom + viewport.x;
    let mut pos_y = node_rect.y * viewport.zoom + viewport.y - offset;
    let mut shift_x = -100.0 * alignment_offset;
    let mut shift_y = -100.0;

    match position {
        Position::Right => {
            pos_x = (node_rect.x + node_rect.width) * viewport.zoom + viewport.x + offset;
            pos_y = (node_rect.y + node_rect.height * alignment_offset) * viewport.zoom + viewport.y;
            shift_x = 0.0;
            shift_y = -100.0 * alignment_offset;
        }
        Position::Bottom => {
            pos_y = (node_rect.y + node_rect.height) * viewport.zoom + viewport.y + offset;
            shift_y = 0.0;
        }
        Position::Left => {
            pos_x = node_rect.x * viewport.zoom + viewport.x - offset;
            pos_y = (node_rect.y + node_rect.height * alignment_offset) * viewport.zoom + viewport.y;
            shift_x = -100.0;
            shift_y = -100.0 * alignment_offset;
        }
        Position::Top => { /* default values used */ }
    }

    format!(
        "translate({}px, {}px) translate({}%, {}%)",
        js_num(pos_x),
        js_num(pos_y),
        js_num(shift_x),
        js_num(shift_y)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::new(x, y, w, h)
    }

    #[test]
    fn top_center_default() {
        let s = get_node_toolbar_transform(
            rect(100.0, 100.0, 50.0, 50.0),
            Viewport::IDENTITY,
            Position::Top,
            10.0,
            Align::Center,
        );
        // Pos: x = (100 + 25)*1 + 0 = 125, y = 100*1 + 0 - 10 = 90
        // Shift: -50%, -100%
        assert_eq!(s, "translate(125px, 90px) translate(-50%, -100%)");
    }

    #[test]
    fn right_start() {
        let s = get_node_toolbar_transform(
            rect(0.0, 0.0, 100.0, 50.0),
            Viewport::IDENTITY,
            Position::Right,
            5.0,
            Align::Start,
        );
        // alignmentOffset=0
        // pos_x = (0 + 100)*1 + 0 + 5 = 105
        // pos_y = (0 + 50*0)*1 + 0 = 0
        // shift_x = 0, shift_y = -0% → js_num collapses to 0%.
        assert_eq!(s, "translate(105px, 0px) translate(0%, 0%)");
    }

    #[test]
    fn bottom_end() {
        let s = get_node_toolbar_transform(
            rect(0.0, 0.0, 100.0, 50.0),
            Viewport::IDENTITY,
            Position::Bottom,
            10.0,
            Align::End,
        );
        // alignmentOffset=1
        // pos_x = (0 + 100*1)*1 = 100
        // pos_y = (0 + 50)*1 + 10 = 60
        // shift_x = -100, shift_y = 0
        assert_eq!(s, "translate(100px, 60px) translate(-100%, 0%)");
    }

    #[test]
    fn applies_viewport_zoom_and_pan() {
        let vp = Viewport::new(50.0, 100.0, 2.0);
        let s = get_node_toolbar_transform(
            rect(10.0, 20.0, 100.0, 50.0),
            vp,
            Position::Top,
            0.0,
            Align::Center,
        );
        // pos_x = (10 + 50)*2 + 50 = 170
        // pos_y = 20*2 + 100 - 0 = 140
        assert_eq!(s, "translate(170px, 140px) translate(-50%, -100%)");
    }
}
