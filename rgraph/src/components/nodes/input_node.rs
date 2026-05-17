//! Port of `xyflow-react/src/components/Nodes/InputNode.tsx`.
//!
//! Status: Phase 6 — implemented with handle.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::geometry::Position;
use rgraph_core::types::handles::HandleType;

use crate::components::handle::Handle;
use crate::types::nodes::{BuiltInNodeData, NodeProps};

/// `input` built-in node: only a source handle on the bottom.
#[component]
pub fn InputNode(props: NodeProps<BuiltInNodeData>) -> Element {
    let label = match &props.data {
        BuiltInNodeData::Labelled { label } => label.clone(),
        BuiltInNodeData::Empty => String::new(),
    };
    let source_position = props.source_position.unwrap_or(Position::Bottom);
    let is_connectable = props.is_connectable.unwrap_or(true);

    rsx! {
        "{label}"
        Handle {
            type_: HandleType::Source,
            position: source_position,
            is_connectable,
        }
    }
}
