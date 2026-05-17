//! Port of `xyflow-react/src/utils/index.ts`.
//!
//! Status: Phase 1 — re-exports active.

pub mod changes;
pub mod general;

pub use changes::{
    apply_edge_changes, apply_node_changes, create_edge_selection_change, create_node_selection_change,
    create_selection_change, edge_to_remove_change, get_elements_diff_changes_edges,
    get_elements_diff_changes_nodes, get_selection_changes_for_edges, get_selection_changes_for_nodes,
    node_to_remove_change,
};
pub use general::{is_edge, is_node, Element, PtrEq};
