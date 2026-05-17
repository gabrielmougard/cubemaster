//! Port of `xyflow-react/src/components/Edges/BaseEdge.tsx`.
//!
//! Status: Phase 6 — implemented.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::utils::general::is_numeric;

use crate::components::edges::edge_text::{EdgeText, EdgeTextProps};
use crate::types::edges::{BaseEdgeProps, EdgeLabelOptions};

#[derive(Props, Clone, PartialEq)]
pub struct BaseEdgeComponentProps {
    pub path: String,
    #[props(default)]
    pub label_x: Option<f64>,
    #[props(default)]
    pub label_y: Option<f64>,
    #[props(default)]
    pub label_options: EdgeLabelOptions,
    #[props(default)]
    pub style: Option<String>,
    #[props(default)]
    pub class_name: Option<String>,
    #[props(default = Some(20.0))]
    pub interaction_width: Option<f64>,
    #[props(default)]
    pub marker_start: Option<String>,
    #[props(default)]
    pub marker_end: Option<String>,
    #[props(default)]
    pub id: Option<String>,
}

/// `<BaseEdge>` — renders the visible SVG `<path>` plus an invisible
/// hit-area path and the optional label.
///
/// Mirrors the TS `BaseEdge`. The `interactionWidth` default is 20
/// (TS line 44).
#[component]
pub fn BaseEdge(props: BaseEdgeComponentProps) -> Element {
    let style_str = props.style.clone().unwrap_or_default();
    let class_str = match &props.class_name {
        Some(extra) => format!("react-flow__edge-path {extra}"),
        None => "react-flow__edge-path".to_string(),
    };
    let marker_start_attr = props.marker_start.clone().unwrap_or_default();
    let marker_end_attr = props.marker_end.clone().unwrap_or_default();
    let id_attr = props.id.clone().unwrap_or_default();

    let interaction_width = props.interaction_width.unwrap_or(20.0);
    let show_label = props
        .label_options
        .label
        .is_some()
        && props.label_x.is_some_and(is_numeric)
        && props.label_y.is_some_and(is_numeric);

    rsx! {
        path {
            id: "{id_attr}",
            d: "{props.path}",
            fill: "none",
            class: "{class_str}",
            style: "{style_str}",
            "marker-start": "{marker_start_attr}",
            "marker-end": "{marker_end_attr}",
        }
        if interaction_width > 0.0 {
            path {
                d: "{props.path}",
                fill: "none",
                "stroke-opacity": "0",
                "stroke-width": "{interaction_width}",
                class: "react-flow__edge-interaction",
            }
        }
        if show_label {
            EdgeText {
                x: props.label_x.unwrap_or(0.0),
                y: props.label_y.unwrap_or(0.0),
                label_options: props.label_options.clone(),
            }
        }
    }
}

// Suppress dead-code on the type-1 `BaseEdgeProps` re-export from
// Phase 1; the component-side props bag carries the same name but
// lives under `components::edges::base_edge::BaseEdgeComponentProps`.
#[allow(dead_code)]
type _BEP = BaseEdgeProps;
#[allow(dead_code)]
type _ETP = EdgeTextProps;
