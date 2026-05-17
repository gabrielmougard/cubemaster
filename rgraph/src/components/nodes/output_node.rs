//! Port of `xyflow-react/src/components/Nodes/OutputNode.tsx`.
//!
//! Status: Phase 6 — implemented with handle.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::geometry::Position;
use rgraph_core::types::handles::HandleType;

use crate::components::handle::Handle;
use crate::types::nodes::{BuiltInNodeData, NodeProps};

/// `output` built-in node: only a target handle on top.
#[component]
pub fn OutputNode(props: NodeProps<BuiltInNodeData>) -> Element {
    let label = match &props.data {
        BuiltInNodeData::Labelled { label } => label.clone(),
        BuiltInNodeData::Empty => String::new(),
    };
    let target_position = props.target_position.unwrap_or(Position::Top);
    let is_connectable = props.is_connectable.unwrap_or(true);

    rsx! {
        Handle {
            type_: HandleType::Target,
            position: target_position,
            is_connectable,
        }
        "{label}"
    }
}
