//! Port of `xyflow-react/src/container/EdgeRenderer/MarkerSymbols.tsx`.
//!
//! Status: Phase 6 — implemented.
//!
//! Provides the SVG content for the two built-in marker symbols
//! (`arrow` and `arrowclosed`). Hosts insert these as children of
//! `<marker>` elements emitted by [`super::marker_definitions`].

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::edges::MarkerType;

#[derive(Props, Clone, PartialEq)]
pub struct MarkerSymbolProps {
    pub type_: MarkerType,
    #[props(default)]
    pub color: Option<String>,
    #[props(default = 1.0)]
    pub stroke_width: f64,
}

/// Render a single marker symbol based on its [`MarkerType`].
#[component]
pub fn MarkerSymbol(props: MarkerSymbolProps) -> Element {
    let stroke_color = props.color.clone().unwrap_or_else(|| "none".to_string());
    match props.type_ {
        MarkerType::Arrow => {
            let style = format!("stroke-width: {}; stroke: {}", props.stroke_width, stroke_color);
            rsx! {
                polyline {
                    class: "arrow",
                    style: "{style}",
                    "stroke-linecap": "round",
                    fill: "none",
                    "stroke-linejoin": "round",
                    points: "-5,-4 0,0 -5,4",
                }
            }
        }
        MarkerType::ArrowClosed => {
            let style = format!(
                "stroke-width: {}; stroke: {}; fill: {}",
                props.stroke_width, stroke_color, stroke_color,
            );
            rsx! {
                polyline {
                    class: "arrowclosed",
                    style: "{style}",
                    "stroke-linecap": "round",
                    "stroke-linejoin": "round",
                    points: "-5,-4 0,0 -5,4 -5,-4",
                }
            }
        }
    }
}
