//! Port of `xyflow-react/src/hooks/useMoveSelectedNodes.ts`.
//!
//! Status: Phase 3 — implemented.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dioxus::prelude::ReadableExt;

use rgraph_core::types::geometry::{Dimensions, XYPosition};
use rgraph_core::types::nodes::NodeDragItem;
use rgraph_core::utils::general::{get_node_dimensions, snap_position};
use rgraph_core::utils::graph::{calculate_node_position, CalculateNodePositionParams};

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;
use crate::types::nodes::Node;

/// Direction & speed parameters for [`MoveSelectedNodes::call`].
///
/// Mirrors the inline `{ direction: XYPosition, factor: number }`
/// argument of the TS hook.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveSelectedNodesParams {
    pub direction: XYPosition,
    pub factor: f64,
}

/// Closure-like handle returned by [`use_move_selected_nodes`].
///
/// `Copy + Clone` are implemented manually to avoid propagating
/// `N: Copy / E: Copy` bounds through the derive.
pub struct MoveSelectedNodes<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    store: RGraphStore<N, E>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy for MoveSelectedNodes<N, E> {}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone for MoveSelectedNodes<N, E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

fn is_selected_and_draggable<D: Clone>(node: &Node<D>, default_draggable: bool) -> bool {
    let selected = node.selected.unwrap_or(false);
    let draggable = node.draggable.unwrap_or(default_draggable);
    selected && draggable
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> MoveSelectedNodes<N, E> {
    /// Compute new positions for all selected+draggable nodes and
    /// dispatch them through `update_node_positions`.
    ///
    /// Mirrors the TS body of `moveSelectedNodes` (lines 19–65).
    pub fn call(&self, params: MoveSelectedNodesParams) {
        let store = self.store;

        let snap_to_grid = *store.snap_to_grid.peek();
        let snap_grid = *store.snap_grid.peek();
        let nodes_draggable = *store.nodes_draggable.peek();
        let node_origin = *store.node_origin.peek();
        let node_extent = *store.node_extent.peek();

        // 5px per key press, unless snap-to-grid is on (TS line 29).
        let x_velo = if snap_to_grid { snap_grid.0 } else { 5.0 };
        let y_velo = if snap_to_grid { snap_grid.1 } else { 5.0 };

        let x_diff = params.direction.x * x_velo * params.factor;
        let y_diff = params.direction.y * y_velo * params.factor;

        let mut updates: HashMap<String, NodeDragItem> = HashMap::new();
        let lookup = store.node_lookup.peek();

        for (id, internal) in lookup.iter() {
            if !is_selected_and_draggable(&internal.user, nodes_draggable) {
                continue;
            }

            let mut next_position = XYPosition {
                x: internal.internals.position_absolute.x + x_diff,
                y: internal.internals.position_absolute.y + y_diff,
            };
            if snap_to_grid {
                next_position = snap_position(next_position, snap_grid);
            }

            let calc = calculate_node_position(CalculateNodePositionParams {
                node_id: id,
                next_position,
                node_lookup: &lookup,
                node_origin,
                node_extent: Some(node_extent),
                on_error: None,
            });

            updates.insert(
                id.clone(),
                NodeDragItem {
                    id: id.clone(),
                    position: calc.position,
                    distance: XYPosition::ZERO,
                    measured: get_node_dimensions(&internal.user),
                    position_absolute: calc.position_absolute,
                    extent: internal.user.extent,
                    parent_id: internal.user.parent_id.clone(),
                    origin: internal.user.origin,
                    expand_parent: internal.user.expand_parent,
                    dragging: Some(false),
                },
            );
        }

        drop(lookup);

        // The TS source mutates `node.position` and
        // `node.internals.positionAbsolute` in place before calling
        // `updateNodePositions`; in our model the action does the
        // computation, so we just dispatch.
        store.update_node_positions(&updates, false);
    }
}

// `Dimensions` import kept because some compiler errors may flag it
// unused on some toolchains; suppress.
#[allow(dead_code)]
type _Dim = Dimensions;

/// Returns a [`MoveSelectedNodes`] handle bound to the current store.
#[must_use]
pub fn use_move_selected_nodes<N, E>() -> MoveSelectedNodes<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    MoveSelectedNodes { store }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use dioxus_signals::{ReadableExt as _, WritableExt as _};
    use std::cell::Cell;

    #[test]
    fn move_selected_nodes_shifts_selected_only() {
        thread_local! {
            static AX: Cell<f64> = const { Cell::new(0.0) };
            static BX: Cell<f64> = const { Cell::new(0.0) };
        }

        #[component]
        fn Probe() -> Element {
            let store = use_rgraph_store::<(), ()>();
            let mover = use_move_selected_nodes::<(), ()>();

            let mut a = Node::<()>::minimal("a", 100.0, 100.0);
            a.selected = Some(true);
            // measured dims so the lookup carries dimensions.
            a.measured = Some(rgraph_core::types::nodes::MeasuredDimensions {
                width: Some(10.0),
                height: Some(10.0),
            });
            let mut b = Node::<()>::minimal("b", 200.0, 200.0);
            b.measured = Some(rgraph_core::types::nodes::MeasuredDimensions {
                width: Some(10.0),
                height: Some(10.0),
            });
            store.has_default_nodes.clone().set(true);
            store.set_nodes(vec![a, b]);

            mover.call(MoveSelectedNodesParams {
                direction: XYPosition::new(1.0, 0.0),
                factor: 1.0,
            });

            // a was selected → moved by +5; b was not → unchanged.
            let nodes = store.nodes.peek().clone();
            for n in &nodes {
                if n.id == "a" {
                    AX.with(|c| c.set(n.position.x));
                } else if n.id == "b" {
                    BX.with(|c| c.set(n.position.x));
                }
            }
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!((AX.with(|c| c.get()) - 105.0).abs() < 1e-9);
        assert!((BX.with(|c| c.get()) - 200.0).abs() < 1e-9);
    }
}
