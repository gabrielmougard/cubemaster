//! Port of `xyflow-react/src/components/Nodes/DefaultNode.tsx`.
//!
//! Status: Phase 5 — implemented (label-only; `<Handle>` integration
//! lands in Phase 6).
//!
//! The TS `DefaultNode` renders a target handle on top, the data
//! label, and a source handle on the bottom. Until Phase 6 mounts the
//! handle component, we emit the label only and document the missing
//! handles with TODOs.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::types::nodes::{BuiltInNodeData, NodeProps};

/// Default built-in node renderer. Mirrors TS
/// `DefaultNode({ data, isConnectable, targetPosition, sourcePosition })`.
///
/// In Phase 5 only the label is rendered. Phase 6 will add the
/// `<Handle type="target" .../>` and `<Handle type="source" .../>`
/// children using `source_position`/`target_position` from the props.
#[component]
pub fn DefaultNode(props: NodeProps<BuiltInNodeData>) -> Element {
    let label = match &props.data {
        BuiltInNodeData::Labelled { label } => label.clone(),
        BuiltInNodeData::Empty => String::new(),
    };

    // TODO(rgraph/phase6):
    //   rsx! {
    //       Handle { type_: HandleType::Target, position: props.target_position.unwrap_or(Position::Top), … }
    //       "{label}"
    //       Handle { type_: HandleType::Source, position: props.source_position.unwrap_or(Position::Bottom), … }
    //   }
    rsx! { "{label}" }
}
