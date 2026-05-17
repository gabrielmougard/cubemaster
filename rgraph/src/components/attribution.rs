//! Port of `xyflow-react/src/components/Attribution/index.tsx`.
//!
//! Status: Phase 4 — implemented.
//!
//! Renders the small "React Flow" link in the bottom-right corner of
//! the viewport. Hidden when `pro_options.hide_attribution = true`.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::viewport::{PanelPosition, ProOptions};

use crate::components::panel::Panel;

/// Props for [`Attribution`]. Mirrors TS `AttributionProps`.
#[derive(Props, Clone, PartialEq)]
pub struct AttributionProps {
    #[props(default)]
    pub pro_options: Option<ProOptions>,
    /// Defaults to `bottom-right`.
    #[props(default)]
    pub position: Option<PanelPosition>,
}

/// Renders the React Flow attribution link unless explicitly hidden
/// via `pro_options.hide_attribution`.
#[component]
pub fn Attribution(props: AttributionProps) -> Element {
    if props
        .pro_options
        .as_ref()
        .is_some_and(|o| o.hide_attribution)
    {
        return rsx! {};
    }

    let position = props.position.unwrap_or(PanelPosition::BottomRight);

    rsx! {
        Panel {
            position: position,
            class_name: "react-flow__attribution".to_string(),
            data_message: "Please only hide this attribution when you are subscribed to React Flow Pro: https://pro.reactflow.dev".to_string(),
            a {
                href: "https://reactflow.dev",
                target: "_blank",
                rel: "noopener noreferrer",
                "aria-label": "React Flow attribution",
                "React Flow"
            }
        }
    }
}
