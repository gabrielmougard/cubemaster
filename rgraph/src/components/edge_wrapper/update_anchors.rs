//! Port of `xyflow-react/src/components/EdgeWrapper/EdgeUpdateAnchors.tsx`.
//!
//! Status: Phase 6 — stub.
//!
//! Renders the two anchor circles at the source/target of a
//! reconnectable edge. Wiring the reconnect gesture requires full
//! `XYHandle` integration which lands in Phase 7.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::geometry::Position;

use crate::components::edges::EdgeAnchor;
use crate::types::edges::Edge;

#[derive(Props, Clone, PartialEq)]
pub struct EdgeUpdateAnchorsProps<E: Clone + PartialEq + 'static = ()> {
    pub edge: Edge<E>,
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub source_position: Position,
    pub target_position: Position,
    #[props(default = 10.0)]
    pub reconnect_radius: f64,
}

/// Phase-6 stub: emits the two anchor circles but doesn't wire the
/// reconnect gesture. Phase 7's `<RGraph>` integration will populate
/// `on_mouse_down` to dispatch through `XYHandle::start` in
/// edge-updater mode.
#[component]
pub fn EdgeUpdateAnchors<E: Clone + PartialEq + 'static>(
    props: EdgeUpdateAnchorsProps<E>,
) -> Element {
    rsx! {
        EdgeAnchor {
            position: props.source_position,
            center_x: props.source_x,
            center_y: props.source_y,
            radius: props.reconnect_radius,
            type_: "source".to_string(),
        }
        EdgeAnchor {
            position: props.target_position,
            center_x: props.target_x,
            center_y: props.target_y,
            radius: props.reconnect_radius,
            type_: "target".to_string(),
        }
    }
}
