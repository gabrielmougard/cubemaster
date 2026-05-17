//! Port of `xyflow-react/src/components/Edges/StepEdge.tsx`.
//!
//! Status: Phase 6 — implemented.
//!
//! Step edge = smoothstep edge with `borderRadius = 0`. The TS source
//! literally renders `<SmoothStepEdge>` with the radius nulled out;
//! we do the same.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::edges::{SmoothStepPathOptions, StepPathOptions};
use rgraph_core::types::geometry::Position;

use crate::components::edges::smoothstep_edge::{SmoothStepEdge, SmoothStepEdgeComponentProps};
use crate::types::edges::EdgeLabelOptions;

#[derive(Props, Clone, PartialEq)]
pub struct StepEdgeComponentProps {
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
    pub path_options: Option<StepPathOptions>,
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
pub fn StepEdge(props: StepEdgeComponentProps) -> Element {
    let offset = props.path_options.as_ref().and_then(|p| p.offset);
    let smoothstep_options = SmoothStepPathOptions {
        border_radius: Some(0.0),
        offset,
        step_position: None,
    };

    rsx! {
        SmoothStepEdge {
            id: props.id.clone(),
            source_x: props.source_x,
            source_y: props.source_y,
            target_x: props.target_x,
            target_y: props.target_y,
            source_position: props.source_position,
            target_position: props.target_position,
            path_options: Some(smoothstep_options),
            label_options: props.label_options.clone(),
            style: props.style.clone(),
            marker_start: props.marker_start.clone(),
            marker_end: props.marker_end.clone(),
            interaction_width: props.interaction_width,
        }
    }
}
