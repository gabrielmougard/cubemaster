//! Port of `xyflow-react/src/components/Nodes/OutputNode.tsx`.
//!
//! Status: Phase 5 — implemented (label-only; `<Handle>` Phase 6).

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::types::nodes::{BuiltInNodeData, NodeProps};

/// `output` built-in node: only a target handle on top (Phase 6).
/// Phase 5 renders just the label.
#[component]
pub fn OutputNode(props: NodeProps<BuiltInNodeData>) -> Element {
    let label = match &props.data {
        BuiltInNodeData::Labelled { label } => label.clone(),
        BuiltInNodeData::Empty => String::new(),
    };
    // TODO(rgraph/phase6): emit a `<Handle type="target" position=…/>`.
    rsx! { "{label}" }
}
