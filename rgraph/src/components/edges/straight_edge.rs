//! Port of `xyflow-react/src/components/Edges/StraightEdge.tsx`.
//!
//! Status: Phase 6 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::utils::edges::straight::{get_straight_path, GetStraightPathParams};

use crate::components::edges::base_edge::{BaseEdge, BaseEdgeComponentProps};
use crate::types::edges::EdgeLabelOptions;

#[derive(Props, Clone, PartialEq)]
pub struct StraightEdgeComponentProps {
    #[props(default)]
    pub id: Option<String>,
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
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
pub fn StraightEdge(props: StraightEdgeComponentProps) -> Element {
    let (path, label_x, label_y, _ox, _oy) = get_straight_path(GetStraightPathParams {
        source_x: props.source_x,
        source_y: props.source_y,
        target_x: props.target_x,
        target_y: props.target_y,
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
