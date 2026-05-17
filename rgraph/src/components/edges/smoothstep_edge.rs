//! Port of `xyflow-react/src/components/Edges/SmoothStepEdge.tsx`.
//!
//! Status: Phase 6 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::edges::SmoothStepPathOptions;
use rgraph_core::types::geometry::Position;
use rgraph_core::utils::edges::smoothstep::{get_smooth_step_path, GetSmoothStepPathParams};

use crate::components::edges::base_edge::{BaseEdge, BaseEdgeComponentProps};
use crate::types::edges::EdgeLabelOptions;

#[derive(Props, Clone, PartialEq)]
pub struct SmoothStepEdgeComponentProps {
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
    pub path_options: Option<SmoothStepPathOptions>,
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
pub fn SmoothStepEdge(props: SmoothStepEdgeComponentProps) -> Element {
    let border_radius = props
        .path_options
        .as_ref()
        .and_then(|p| p.border_radius)
        .unwrap_or(5.0);
    let offset = props
        .path_options
        .as_ref()
        .and_then(|p| p.offset)
        .unwrap_or(20.0);
    let step_position = props
        .path_options
        .as_ref()
        .and_then(|p| p.step_position)
        .unwrap_or(0.5);

    let (path, label_x, label_y, _ox, _oy) = get_smooth_step_path(GetSmoothStepPathParams {
        source_x: props.source_x,
        source_y: props.source_y,
        source_position: props.source_position,
        target_x: props.target_x,
        target_y: props.target_y,
        target_position: props.target_position,
        border_radius,
        center_x: None,
        center_y: None,
        offset,
        step_position,
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
