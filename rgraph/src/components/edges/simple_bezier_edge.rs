//! Port of `xyflow-react/src/components/Edges/SimpleBezierEdge.tsx`.
//!
//! Status: Phase 6 — implemented.
//!
//! `getSimpleBezierPath` is a TS-only utility (not in `rgraph_core`),
//! so we provide it in this module alongside the component.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::geometry::Position;
use rgraph_core::utils::edges::bezier::get_bezier_edge_center;

use crate::components::edges::base_edge::{BaseEdge, BaseEdgeComponentProps};
use crate::types::edges::EdgeLabelOptions;

/// Parameters for [`get_simple_bezier_path`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GetSimpleBezierPathParams {
    pub source_x: f64,
    pub source_y: f64,
    pub source_position: Position,
    pub target_x: f64,
    pub target_y: f64,
    pub target_position: Position,
}

fn get_control(pos: Position, x1: f64, y1: f64, x2: f64, y2: f64) -> (f64, f64) {
    match pos {
        Position::Left | Position::Right => (0.5 * (x1 + x2), y1),
        _ => (x1, 0.5 * (y1 + y2)),
    }
}

/// `getSimpleBezierPath` — returns `(path, labelX, labelY, offsetX, offsetY)`.
///
/// Mirrors the TS export with the same name. Used by the
/// [`SimpleBezierEdge`] component and by
/// `<ConnectionLine type=SimpleBezier>`.
#[must_use]
pub fn get_simple_bezier_path(p: GetSimpleBezierPathParams) -> (String, f64, f64, f64, f64) {
    let (s_cx, s_cy) = get_control(p.source_position, p.source_x, p.source_y, p.target_x, p.target_y);
    let (t_cx, t_cy) = get_control(p.target_position, p.target_x, p.target_y, p.source_x, p.source_y);
    let (label_x, label_y, off_x, off_y) = get_bezier_edge_center(
        p.source_x, p.source_y, p.target_x, p.target_y, s_cx, s_cy, t_cx, t_cy,
    );
    let path = format!(
        "M{sx},{sy} C{scx},{scy} {tcx},{tcy} {tx},{ty}",
        sx = p.source_x,
        sy = p.source_y,
        scx = s_cx,
        scy = s_cy,
        tcx = t_cx,
        tcy = t_cy,
        tx = p.target_x,
        ty = p.target_y,
    );
    (path, label_x, label_y, off_x, off_y)
}

#[derive(Props, Clone, PartialEq)]
pub struct SimpleBezierEdgeComponentProps {
    #[props(default)]
    pub id: Option<String>,
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
    #[props(default = Position::Bottom)]
    pub source_position: Position,
    #[props(default = Position::Top)]
    pub target_position: Position,
    #[props(default)]
    pub label_options: EdgeLabelOptions,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub marker_start: Option<String>,
    #[props(default)]
    pub marker_end: Option<String>,
    #[props(default)]
    pub interaction_width: Option<f64>,
}

#[component]
pub fn SimpleBezierEdge(props: SimpleBezierEdgeComponentProps) -> Element {
    let (path, label_x, label_y, _ox, _oy) = get_simple_bezier_path(GetSimpleBezierPathParams {
        source_x: props.source_x,
        source_y: props.source_y,
        source_position: props.source_position,
        target_x: props.target_x,
        target_y: props.target_y,
        target_position: props.target_position,
    });

    rsx! {
        BaseEdge {
            id: props.id.clone(),
            path,
            label_x: Some(label_x),
            label_y: Some(label_y),
            label_options: props.label_options.clone(),
            style: props.style.clone(),
            marker_start: props.marker_start.clone(),
            marker_end: props.marker_end.clone(),
            interaction_width: props.interaction_width,
        }
    }
}
