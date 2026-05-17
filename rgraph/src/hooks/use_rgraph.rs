//! Port of `xyflow-react/src/hooks/useReactFlow.ts`.
//!
//! Status: Phase 3 — partial implementation.
//!
//! In TS the hook returns a `ReactFlowInstance<NodeType, EdgeType>` —
//! a large bundle composing `GeneralHelpers`, `ViewportHelperFunctions`
//! and `viewportInitialized`. Composing those in Rust requires the
//! callbacks half of [`crate::types::instance::GeneralHelpers`] to be
//! filled in, which depends on the Phase 6 `BatchProvider` for the
//! `nodeQueue`/`edgeQueue` plumbing.
//!
//! For Phase 3 we expose [`use_rgraph`] returning an [`RGraphHandle`]
//! that bundles the store, the [`crate::hooks::use_viewport_helper::ViewportHelper`],
//! and the convenience methods that don't need the batch queue
//! (`get_nodes`, `set_nodes`, `add_nodes`, `get_node`,
//! `get_internal_node`, `get_edges`, `set_edges`, `add_edges`,
//! `get_edge`, `to_object`, `update_node`, `update_node_data`,
//! `update_edge`, `update_edge_data`, `get_nodes_bounds`,
//! `get_node_connections`, `get_handle_connections`,
//! `is_node_intersecting`, `get_intersecting_nodes`).
//!
//! Phase 7 will rewrap this handle into the full [`crate::types::instance::RGraphInstance`]
//! struct of `Callback`s.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dioxus::prelude::ReadableExt;

use rgraph_core::types::geometry::Rect;
use rgraph_core::types::handles::HandleType;
use rgraph_core::utils::general::{evaluate_absolute_position, get_node_dimensions, get_overlapping_area};
use rgraph_core::utils::graph::{get_nodes_bounds as core_get_nodes_bounds, GetNodesBoundsParams, NodeOrId};

use crate::context::use_rgraph_store;
use crate::hooks::use_viewport_helper::{use_viewport_helper, ViewportHelper};
use crate::store::RGraphStore;
use crate::types::edges::Edge;
use crate::types::instance::{
    DeleteElementsOptions, DeletedElements, GetHandleConnectionsArgs, GetNodeConnectionsArgs,
    NodeOrIdOrInternal, NodeOrRect, NodePartial, RGraphJsonObject, UpdateOptions,
};
use crate::types::nodes::{InternalNode, Node};

/// Phase-3 imperative graph handle returned by [`use_rgraph`].
///
/// `Copy + Clone` because every field is itself `Copy`. We implement
/// these manually because `#[derive(Copy, Clone)]` would propagate
/// `N: Copy + E: Copy` bounds, which is too restrictive — the handle
/// only stores `Signal`-backed copies, not actual values.
pub struct RGraphHandle<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    pub store: RGraphStore<N, E>,
    pub viewport: ViewportHelper<N, E>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Copy for RGraphHandle<N, E> {}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> Clone for RGraphHandle<N, E> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> RGraphHandle<N, E> {
    /// `true` once a `PanZoomInstance` has been mounted on the store.
    /// Mirrors TS `viewportInitialized` (TS line 65).
    pub fn viewport_initialized(&self) -> bool {
        self.store.pan_zoom.peek().is_some()
    }

    // -- Node / edge accessors --------------------------------------------

    /// Returns a `Vec` clone of the current nodes.
    pub fn get_nodes(&self) -> Vec<Node<N>> {
        self.store.nodes.peek().clone()
    }

    /// Replace the node list. Mirrors TS `setNodes(payload)`.
    pub fn set_nodes(&self, nodes: Vec<Node<N>>) {
        self.store.set_nodes(nodes);
    }

    /// Append nodes to the current list. Mirrors TS `addNodes(payload)`.
    pub fn add_nodes(&self, mut new_nodes: Vec<Node<N>>) {
        let mut current = self.store.nodes.peek().clone();
        current.append(&mut new_nodes);
        self.store.set_nodes(current);
    }

    /// Add a single node. Convenience wrapper over [`Self::add_nodes`].
    pub fn add_node(&self, node: Node<N>) {
        self.add_nodes(vec![node]);
    }

    /// Returns the node with the given id, or `None`.
    pub fn get_node(&self, id: &str) -> Option<Node<N>> {
        self.store
            .node_lookup
            .peek()
            .get(id)
            .map(|n| n.user.clone())
    }

    /// Returns the [`InternalNode`] with the given id, or `None`.
    pub fn get_internal_node(&self, id: &str) -> Option<InternalNode<N>> {
        self.store.node_lookup.peek().get(id).cloned()
    }

    /// Returns a `Vec` clone of the current edges.
    pub fn get_edges(&self) -> Vec<Edge<E>> {
        self.store.edges.peek().clone()
    }

    /// Replace the edge list. Mirrors TS `setEdges(payload)`.
    pub fn set_edges(&self, edges: Vec<Edge<E>>) {
        self.store.set_edges(edges);
    }

    /// Append edges to the current list.
    pub fn add_edges(&self, mut new_edges: Vec<Edge<E>>) {
        let mut current = self.store.edges.peek().clone();
        current.append(&mut new_edges);
        self.store.set_edges(current);
    }

    /// Add a single edge.
    pub fn add_edge(&self, edge: Edge<E>) {
        self.add_edges(vec![edge]);
    }

    /// Returns the edge with the given id, or `None`.
    pub fn get_edge(&self, id: &str) -> Option<Edge<E>> {
        self.store.edge_lookup.peek().get(id).cloned()
    }

    /// Snapshot of the flow as a JSON-shaped object.
    /// Mirrors TS `toObject`.
    pub fn to_object(&self) -> RGraphJsonObject<N, E> {
        let nodes = self.get_nodes();
        let edges = self.get_edges();
        let viewport = self.viewport.get_viewport();
        RGraphJsonObject { nodes, edges, viewport }
    }

    // -- Update helpers (merge / replace) ---------------------------------

    /// Update a node by id with a partial overlay. Mirrors TS
    /// `updateNode(id, nodeUpdate, options?)` for the data-only
    /// branch; the function-overload variant is omitted (callers can
    /// always pre-compute the partial themselves).
    pub fn update_node(&self, id: &str, partial: NodePartial<N>, options: UpdateOptions) {
        let mut nodes = self.get_nodes();
        for node in nodes.iter_mut() {
            if node.id == id {
                apply_node_partial(node, partial.clone(), options.replace);
                break;
            }
        }
        self.set_nodes(nodes);
    }

    /// Update a node's `data` field. Mirrors TS `updateNodeData`.
    pub fn update_node_data(&self, id: &str, data: N, _options: UpdateOptions) {
        // The TS source distinguishes `replace` from merge by spreading
        // `node.data` before the new fields. Our `D` is opaque, so we
        // can't deep-merge generically; both branches degenerate to a
        // wholesale write. Phase 7 may introduce a `MergeData` trait
        // for fine-grained merges if required.
        let mut nodes = self.get_nodes();
        for node in nodes.iter_mut() {
            if node.id == id {
                node.data = data;
                break;
            }
        }
        self.set_nodes(nodes);
    }

    /// Update an edge's `data` field. Mirrors TS `updateEdgeData`.
    pub fn update_edge_data(&self, id: &str, data: E, _options: UpdateOptions) {
        let mut edges = self.get_edges();
        for edge in edges.iter_mut() {
            if edge.id == id {
                edge.data = data;
                break;
            }
        }
        self.set_edges(edges);
    }

    // -- Geometry helpers -------------------------------------------------

    /// Returns the bounding rect of the supplied nodes / ids.
    /// Mirrors TS `getNodesBounds`.
    pub fn get_nodes_bounds(&self, nodes: Vec<NodeOrIdOrInternal<N>>) -> Rect {
        let lookup = self.store.node_lookup.peek();
        let node_origin = *self.store.node_origin.peek();

        // Convert to the core `NodeOrId<'_, D>` shape. Since core uses
        // borrowed references and we have owned values here, we need
        // to carry them in side-buffers to keep them alive while we
        // build the iterator.
        let owned_nodes: Vec<Node<N>> = nodes
            .iter()
            .filter_map(|n| match n {
                NodeOrIdOrInternal::Node(n) => Some(n.clone()),
                _ => None,
            })
            .collect();
        let owned_internals: Vec<InternalNode<N>> = nodes
            .iter()
            .filter_map(|n| match n {
                NodeOrIdOrInternal::Internal(i) => Some(i.clone()),
                _ => None,
            })
            .collect();
        let owned_ids: Vec<String> = nodes
            .iter()
            .filter_map(|n| match n {
                NodeOrIdOrInternal::Id(s) => Some(s.clone()),
                _ => None,
            })
            .collect();

        let refs: Vec<NodeOrId<'_, N>> = owned_nodes
            .iter()
            .map(NodeOrId::Node)
            .chain(owned_internals.iter().map(NodeOrId::Internal))
            .chain(owned_ids.iter().map(|s| NodeOrId::Id(s.as_str())))
            .collect();

        core_get_nodes_bounds(
            refs,
            GetNodesBoundsParams {
                node_origin,
                node_lookup: Some(&lookup),
            },
        )
    }

    /// True iff the supplied node / rect intersects with `area`.
    /// Mirrors TS `isNodeIntersecting`.
    pub fn is_node_intersecting(&self, node_or_rect: NodeOrRect<N>, area: Rect, partially: bool) -> bool {
        let Some(node_rect) = self.resolve_node_rect(&node_or_rect) else {
            return false;
        };
        let overlap = get_overlapping_area(node_rect, area);
        if partially && overlap > 0.0 {
            return true;
        }
        overlap >= node_rect.width * node_rect.height || overlap >= area.width * area.height
    }

    /// Returns nodes intersecting with the supplied node / rect.
    /// Mirrors TS `getIntersectingNodes`.
    pub fn get_intersecting_nodes(
        &self,
        node_or_rect: NodeOrRect<N>,
        partially: bool,
        nodes: Option<Vec<Node<N>>>,
    ) -> Vec<Node<N>> {
        let Some(node_rect) = self.resolve_node_rect(&node_or_rect) else {
            return Vec::new();
        };
        let lookup = self.store.node_lookup.peek();
        let candidates = nodes.unwrap_or_else(|| self.get_nodes());

        let exclude_id: Option<&str> = match &node_or_rect {
            NodeOrRect::Node(n) => Some(n.id.as_str()),
            _ => None,
        };

        candidates
            .into_iter()
            .filter(|n| {
                if Some(n.id.as_str()) == exclude_id {
                    return false;
                }
                let internal = lookup.get(&n.id);
                let cur_rect = match internal {
                    Some(i) => Rect {
                        x: i.internals.position_absolute.x,
                        y: i.internals.position_absolute.y,
                        width: get_node_dimensions(i).width,
                        height: get_node_dimensions(i).height,
                    },
                    None => Rect {
                        x: n.position.x,
                        y: n.position.y,
                        width: get_node_dimensions(n).width,
                        height: get_node_dimensions(n).height,
                    },
                };
                let overlap = get_overlapping_area(cur_rect, node_rect);
                if partially && overlap > 0.0 {
                    return true;
                }
                overlap >= cur_rect.width * cur_rect.height
                    || overlap >= node_rect.width * node_rect.height
            })
            .collect()
    }

    fn resolve_node_rect(&self, n: &NodeOrRect<N>) -> Option<Rect> {
        match n {
            NodeOrRect::Rect(r) => Some(*r),
            NodeOrRect::Id(id) => {
                let lookup = self.store.node_lookup.peek();
                let internal = lookup.get(id)?;
                Some(Rect {
                    x: internal.internals.position_absolute.x,
                    y: internal.internals.position_absolute.y,
                    width: get_node_dimensions(internal).width,
                    height: get_node_dimensions(internal).height,
                })
            }
            NodeOrRect::Node(node) => {
                let lookup = self.store.node_lookup.peek();
                let node_origin = *self.store.node_origin.peek();
                let position = match &node.parent_id {
                    Some(pid) => evaluate_absolute_position(
                        node.position,
                        get_node_dimensions(node),
                        pid,
                        &lookup,
                        node_origin,
                    ),
                    None => node.position,
                };
                let dim = get_node_dimensions(node);
                Some(Rect {
                    x: position.x,
                    y: position.y,
                    width: dim.width,
                    height: dim.height,
                })
            }
        }
    }

    // -- Connection lookup -----------------------------------------------

    /// Returns the [`HandleConnection`]s for a given handle.
    /// Mirrors TS `getHandleConnections`.
    pub fn get_handle_connections(
        &self,
        args: GetHandleConnectionsArgs,
    ) -> Vec<rgraph_core::types::connection::HandleConnection> {
        let key = match (&args.type_, &args.id) {
            (HandleType::Source, Some(id)) => format!("{}-source-{id}", args.node_id),
            (HandleType::Target, Some(id)) => format!("{}-target-{id}", args.node_id),
            (HandleType::Source, None) => format!("{}-source", args.node_id),
            (HandleType::Target, None) => format!("{}-target", args.node_id),
        };
        self.store
            .connection_lookup
            .peek()
            .get(&key)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns the [`NodeConnection`]s for a given node, optionally
    /// filtered by handle type / handle id.
    pub fn get_node_connections(
        &self,
        args: GetNodeConnectionsArgs,
    ) -> Vec<rgraph_core::types::connection::NodeConnection> {
        let key = match (&args.type_, &args.handle_id) {
            (Some(HandleType::Source), Some(id)) => format!("{}-source-{id}", args.node_id),
            (Some(HandleType::Target), Some(id)) => format!("{}-target-{id}", args.node_id),
            (Some(HandleType::Source), None) => format!("{}-source", args.node_id),
            (Some(HandleType::Target), None) => format!("{}-target", args.node_id),
            (None, _) => args.node_id.clone(),
        };
        self.store
            .connection_lookup
            .peek()
            .get(&key)
            .map(|m| {
                m.values()
                    .map(|hc| rgraph_core::types::connection::NodeConnection {
                        connection: hc.connection.clone(),
                        edge_id: hc.edge_id.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    // -- Delete (synchronous Phase 3 stub) -------------------------------

    /// Synchronous delete. Mirrors the TS `deleteElements` call but
    /// without the `onBeforeDelete` async-hook plumbing — Phase 7 will
    /// add that. For Phase 3 the supplied ids are removed directly via
    /// the change pipeline, mirroring what TS does once
    /// `getElementsToRemove` has resolved.
    pub fn delete_elements(&self, options: DeleteElementsOptions<N, E>) -> DeletedElements<N, E> {
        use crate::utils::changes::{edge_to_remove_change, node_to_remove_change};

        let nodes_to_remove: Vec<String> = options
            .nodes
            .unwrap_or_default()
            .iter()
            .map(|r| r.id().to_string())
            .collect();
        let edges_to_remove: Vec<String> = options
            .edges
            .unwrap_or_default()
            .iter()
            .map(|r| r.id().to_string())
            .collect();

        let nodes_snapshot = self.get_nodes();
        let edges_snapshot = self.get_edges();

        let deleted_nodes: Vec<Node<N>> = nodes_snapshot
            .iter()
            .filter(|n| nodes_to_remove.contains(&n.id))
            .cloned()
            .collect();
        let deleted_edges: Vec<Edge<E>> = edges_snapshot
            .iter()
            .filter(|e| edges_to_remove.contains(&e.id))
            .cloned()
            .collect();

        if !deleted_edges.is_empty() {
            if let Some(handler) = *self.store.on_edges_delete.peek() {
                handler.call(deleted_edges.clone());
            }
            self.store
                .trigger_edge_changes(deleted_edges.iter().map(edge_to_remove_change).collect());
        }
        if !deleted_nodes.is_empty() {
            if let Some(handler) = *self.store.on_nodes_delete.peek() {
                handler.call(deleted_nodes.clone());
            }
            self.store
                .trigger_node_changes(deleted_nodes.iter().map(node_to_remove_change).collect());
        }

        if let Some(handler) = *self.store.on_delete.peek()
            && (!deleted_nodes.is_empty() || !deleted_edges.is_empty())
        {
            handler.call(crate::types::general::OnDeleteArgs {
                nodes: deleted_nodes.clone(),
                edges: deleted_edges.clone(),
            });
        }

        DeletedElements {
            deleted_nodes,
            deleted_edges,
        }
    }
}

fn apply_node_partial<D: Clone>(node: &mut Node<D>, partial: NodePartial<D>, replace: bool) {
    if replace {
        if let Some(p) = partial.position { node.position = p; }
        if let Some(d) = partial.data { node.data = d; }
        node.selected = partial.selected;
        node.hidden = partial.hidden;
        node.dragging = partial.dragging;
        node.draggable = partial.draggable;
        node.selectable = partial.selectable;
        node.connectable = partial.connectable;
        node.deletable = partial.deletable;
        node.width = partial.width;
        node.height = partial.height;
        node.parent_id = partial.parent_id;
        node.z_index = partial.z_index;
        node.aria_label = partial.aria_label;
        node.type_ = partial.type_;
    } else {
        if let Some(p) = partial.position { node.position = p; }
        if let Some(d) = partial.data { node.data = d; }
        if partial.selected.is_some() { node.selected = partial.selected; }
        if partial.hidden.is_some() { node.hidden = partial.hidden; }
        if partial.dragging.is_some() { node.dragging = partial.dragging; }
        if partial.draggable.is_some() { node.draggable = partial.draggable; }
        if partial.selectable.is_some() { node.selectable = partial.selectable; }
        if partial.connectable.is_some() { node.connectable = partial.connectable; }
        if partial.deletable.is_some() { node.deletable = partial.deletable; }
        if partial.width.is_some() { node.width = partial.width; }
        if partial.height.is_some() { node.height = partial.height; }
        if partial.parent_id.is_some() { node.parent_id = partial.parent_id; }
        if partial.z_index.is_some() { node.z_index = partial.z_index; }
        if partial.aria_label.is_some() { node.aria_label = partial.aria_label; }
        if partial.type_.is_some() { node.type_ = partial.type_; }
    }
}

// HashMap stays as a dependency for future expansion.
#[allow(dead_code)]
type _Map = HashMap<String, ()>;

/// Returns a [`RGraphHandle`] for the current store. Mirrors TS
/// `useReactFlow()`.
#[must_use]
pub fn use_rgraph<N, E>() -> RGraphHandle<N, E>
where
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
{
    let store = use_rgraph_store::<N, E>();
    let viewport = use_viewport_helper::<N, E>();
    RGraphHandle { store, viewport }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use dioxus::prelude::*;
    use std::cell::Cell;

    #[test]
    fn get_set_add_nodes_round_trip() {
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            let h = use_rgraph::<(), ()>();
            h.add_node(Node::<()>::minimal("a", 0.0, 0.0));
            h.add_nodes(vec![
                Node::<()>::minimal("b", 1.0, 1.0),
                Node::<()>::minimal("c", 2.0, 2.0),
            ]);
            COUNT.with(|c| c.set(h.get_nodes().len()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(COUNT.with(|c| c.get()), 3);
    }

    #[test]
    fn delete_elements_removes_only_supplied_ids() {
        thread_local! { static COUNT: Cell<usize> = const { Cell::new(0) }; }

        #[component]
        fn Probe() -> Element {
            use dioxus_signals::WritableExt;
            let h = use_rgraph::<(), ()>();
            h.store.has_default_nodes.clone().set(true);
            h.set_nodes(vec![
                Node::<()>::minimal("a", 0.0, 0.0),
                Node::<()>::minimal("b", 1.0, 1.0),
            ]);
            h.delete_elements(DeleteElementsOptions {
                nodes: Some(vec![crate::types::instance::NodeRef::Id("a".into())]),
                edges: None,
            });
            COUNT.with(|c| c.set(h.get_nodes().len()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(COUNT.with(|c| c.get()), 1);
    }

    #[test]
    fn viewport_initialized_is_false_in_phase3() {
        thread_local! { static OK: Cell<bool> = const { Cell::new(true) }; }

        #[component]
        fn Probe() -> Element {
            let h = use_rgraph::<(), ()>();
            OK.with(|c| c.set(h.viewport_initialized()));
            rsx! { div {} }
        }
        fn Root() -> Element {
            rsx! { RGraphProvider::<(), ()> { Probe {} } }
        }
        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(!OK.with(|c| c.get()));
    }
}
