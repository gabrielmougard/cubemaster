//! Port of `xyflow-react/src/components/Edges/BezierEdge.tsx`.
//!
//! Status: Phase 6 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::edges::BezierPathOptions;
use rgraph_core::types::geometry::Position;
use rgraph_core::utils::edges::bezier::{get_bezier_path, GetBezierPathParams};

use crate::components::edges::base_edge::{BaseEdge, BaseEdgeComponentProps};
use crate::types::edges::EdgeLabelOptions;

#[derive(Props, Clone, PartialEq)]
pub struct BezierEdgeComponentProps {
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
    pub path_options: Option<BezierPathOptions>,
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
pub fn BezierEdge(props: BezierEdgeComponentProps) -> Element {
    let curvature = props
        .path_options
        .as_ref()
        .and_then(|p| p.curvature)
        .unwrap_or(0.25);

    let (path, label_x, label_y, _ox, _oy) = get_bezier_path(GetBezierPathParams {
        source_x: props.source_x,
        source_y: props.source_y,
        source_position: props.source_position,
        target_x: props.target_x,
        target_y: props.target_y,
        target_position: props.target_position,
        curvature,
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
