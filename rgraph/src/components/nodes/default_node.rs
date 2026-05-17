//! Port of `xyflow-react/src/components/Nodes/DefaultNode.tsx`.
//!
//! Status: Phase 6 — implemented with handles.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use rgraph_core::types::geometry::Position;
use rgraph_core::types::handles::HandleType;

use crate::components::handle::Handle;
use crate::types::nodes::{BuiltInNodeData, NodeProps};

/// Default built-in node renderer. Mirrors TS
/// `DefaultNode({ data, isConnectable, targetPosition, sourcePosition })`:
/// a target handle on top, the data label, and a source handle on the
/// bottom.
#[component]
pub fn DefaultNode(props: NodeProps<BuiltInNodeData>) -> Element {
    let label = match &props.data {
        BuiltInNodeData::Labelled { label } => label.clone(),
        BuiltInNodeData::Empty => String::new(),
    };
    let target_position = props.target_position.unwrap_or(Position::Top);
    let source_position = props.source_position.unwrap_or(Position::Bottom);
    let is_connectable = props.is_connectable.unwrap_or(true);

    rsx! {
        Handle {
            type_: HandleType::Target,
            position: target_position,
            is_connectable,
        }
        "{label}"
        Handle {
            type_: HandleType::Source,
            position: source_position,
            is_connectable,
        }
    }
}
