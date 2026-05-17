//! Port of `xyflow-react/src/components/Nodes/utils.ts`.
//!
//! Status: Phase 5 — implemented.
//!
//! `handle_node_click(...)` — toggles a node's selection state when
//! the user clicks it. Called from:
//!
//! 1. The `NodeWrapper`'s onClick handler (when the node isn't
//!    draggable, or `selectNodesOnDrag = false`, or `nodeDragThreshold > 0`).
//! 2. The drag-start handler when the node is draggable and
//!    `selectNodesOnDrag = true` (Phase 5+).

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::ReadableExt;

use crate::store::RGraphStore;
use crate::types::general::UnselectNodesAndEdgesParams;
use crate::types::nodes::Node;

/// Args for [`handle_node_click`]. Mirrors the TS object-arg shape.
pub struct HandleNodeClickArgs<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    pub id: String,
    pub store: RGraphStore<N, E>,
    /// When `true`, force-unselect the node even if it was already
    /// selected. Mapped from the TS `Escape` key press.
    pub unselect: bool,
}

/// Toggle node selection in response to a click.
///
/// Mirrors the TS `handleNodeClick`. The order of operations is:
///
/// 1. Clear `nodes_selection_active`.
/// 2. If the node isn't selected → add it to the selection.
/// 3. If it is selected AND (`unselect == true` OR
///    `multi_selection_active == true`) → unselect it.
///
/// The TS implementation also `blur`s the node element via
/// `requestAnimationFrame`; Phase 5 doesn't have a node ref handle to
/// blur, so we omit that side-effect. It mainly matters for keyboard
/// flow and will be reinstated when handles land in Phase 6.
pub fn handle_node_click<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static>(
    args: HandleNodeClickArgs<N, E>,
) {
    let HandleNodeClickArgs { id, store, unselect } = args;
    let Some(node) = store.node_lookup.peek().get(&id).cloned() else {
        // TS calls `onError?.('012', errorMessages['error012'](id))` —
        // we forward through the `on_error` callback when present.
        if let Some(handler) = *store.on_error.peek() {
            handler.call(crate::types::component_props::OnErrorArgs {
                id: "012".to_string(),
                message: format!(
                    "[rgraph]: The node with id \"{id}\" does not exist. \
                     If you are using a custom node, make sure you have set the `id` correctly."
                ),
            });
        }
        return;
    };

    use dioxus::prelude::WritableExt;
    store.nodes_selection_active.clone().set(false);

    let multi = *store.multi_selection_active.peek();
    let selected = node.user.selected.unwrap_or(false);

    if !selected {
        store.add_selected_nodes(vec![id.clone()]);
    } else if unselect || (selected && multi) {
        let unsel_params: UnselectNodesAndEdgesParams<N, E> = UnselectNodesAndEdgesParams {
            nodes: Some(vec![Node {
                id: id.clone(),
                ..node.user.clone()
            }]),
            edges: Some(Vec::new()),
        };
        store.unselect_nodes_and_edges(unsel_params);
    }
}
