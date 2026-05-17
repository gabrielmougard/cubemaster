//! Port of `xyflow-react/src/components/Nodes/GroupNode.tsx`.
//!
//! Status: Phase 5 — implemented.
//!
//! The TS `GroupNode` literally returns `null`; group nodes are just
//! a styled box hosting children laid out by the `parent_id` relation.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::*;

use crate::types::nodes::{BuiltInNodeData, NodeProps};

/// `group` built-in node — renders nothing inside the wrapper.
#[component]
pub fn GroupNode(props: NodeProps<BuiltInNodeData>) -> Element {
    let _ = props;
    rsx! {}
}
