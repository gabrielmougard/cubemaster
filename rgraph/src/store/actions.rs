//! Action methods of [`crate::store::RGraphStore`].
//!
//! Status: Phase 2 — implemented.
//!
//! TS reference: the body of `createStore` in
//! `xyflow-react/src/store/index.ts` (lines 55–452).
//!
//! Each public action below mirrors a method exported by the TS Zustand
//! store. They are implemented as `&self` methods on [`RGraphStore`]
//! (which is `Copy`), so callers can capture the store in closures and
//! invoke the actions ergonomically.
//!
//! Pieces that require a live `PanZoomInstance` (mainly `pan_by` and
//! `set_center`) compile but no-op when `pan_zoom` is still `None` —
//! they will be fully wired in Phase 4 once the `ZoomPane` mounts the
//! `rgraph_zoom` engine.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashSet;

use dioxus::prelude::{Readable, ReadableExt, Writable, WritableExt};

use rgraph_core::types::changes::{EdgeChange, NodeChange};
use rgraph_core::types::connection::{initial_connection, ConnectionState};
use rgraph_core::types::geometry::{CoordinateExtent, XYPosition};
use rgraph_core::types::nodes::{InternalNode, NodeOrigin, ParentLookup};
use rgraph_core::types::viewport::Viewport;
use rgraph_core::utils::store::{
    adopt_user_nodes, update_absolute_positions, update_connection_lookup, AdoptUserNodesOptions,
    UpdateNodesOptions,
};

use crate::store::{initial_state, InitialStateParams, RGraphStore};
use crate::types::edges::Edge;
use crate::types::general::UnselectNodesAndEdgesParams;
use crate::types::nodes::Node;
use crate::utils::changes::{
    apply_edge_changes, apply_node_changes, create_edge_selection_change, create_node_selection_change,
    get_selection_changes_for_edges, get_selection_changes_for_nodes,
};

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> RGraphStore<N, E> {
    // ---------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------

    /// Read the current `UpdateNodesOptions` view from the signal-backed
    /// configuration fields. Used by the actions that delegate to
    /// `rgraph_core::utils::store` helpers.
    fn current_update_options(&self) -> UpdateNodesOptions {
        UpdateNodesOptions {
            node_origin: *self.node_origin.peek(),
            node_extent: *self.node_extent.peek(),
            elevate_nodes_on_select: *self.elevate_nodes_on_select.peek(),
            z_index_mode: *self.z_index_mode.peek(),
        }
    }

    // ---------------------------------------------------------------------
    // `set_nodes`, `set_edges`, `set_default_nodes_and_edges`
    // ---------------------------------------------------------------------

    /// Replace the node list.
    ///
    /// Mirrors the TS `setNodes` (lines 99–141). Calls `adopt_user_nodes`
    /// to refresh the lookups, updates the `nodes_initialized` flag,
    /// and toggles `nodes_selection_active` off if no nodes are
    /// selected after the update.
    pub fn set_nodes(&self, mut nodes: Vec<Node<N>>) {
        let options = AdoptUserNodesOptions {
            base: self.current_update_options(),
            check_equality: true,
        };

        // adopt_user_nodes needs `&mut` access to the lookup signals.
        let (nodes_initialized, has_selected_nodes) = self
            .node_lookup
            .write_unchecked()
            .with_combined(&mut *self.parent_lookup.write_unchecked(), &nodes, &options);

        let nodes_selection_active =
            *self.nodes_selection_active.peek() && has_selected_nodes;

        // Apply state updates.
        let fit_view_queued = *self.fit_view_queued.peek();
        self.nodes_initialized.clone().set(nodes_initialized);
        self.nodes_selection_active
            .clone()
            .set(nodes_selection_active);

        // Reuse the now-mutated lookup; `nodes` carries the public list.
        std::mem::swap(&mut nodes, &mut self.nodes.clone().write());

        if fit_view_queued && nodes_initialized {
            self.fit_view_queued.clone().set(false);
            self.fit_view_options.clone().set(None);
            // Actual `resolveFitView` is hooked in Phase 4.
        }
    }

    /// Replace the edge list and refresh `connection_lookup` /
    /// `edge_lookup`. Mirrors TS `setEdges` (lines 142–148).
    pub fn set_edges(&self, edges: Vec<Edge<E>>) {
        {
            let mut connection_lookup = self.connection_lookup.clone().write_unchecked();
            let mut edge_lookup = self.edge_lookup.clone().write_unchecked();
            update_connection_lookup(&mut connection_lookup, &mut edge_lookup, &edges);
        }
        self.edges.clone().set(edges);
    }

    /// Set the default (uncontrolled) nodes / edges. Mirrors TS
    /// `setDefaultNodesAndEdges` (lines 149–160).
    pub fn set_default_nodes_and_edges(
        &self,
        nodes: Option<Vec<Node<N>>>,
        edges: Option<Vec<Edge<E>>>,
    ) {
        if let Some(nodes) = nodes {
            self.set_nodes(nodes);
            self.has_default_nodes.clone().set(true);
        }
        if let Some(edges) = edges {
            self.set_edges(edges);
            self.has_default_edges.clone().set(true);
        }
    }

    // ---------------------------------------------------------------------
    // `update_node_positions`
    // ---------------------------------------------------------------------

    /// Apply a batch of node-drag positions to the store. Mirrors TS
    /// `updateNodePositions` (lines 210–263).
    ///
    /// The dragging cursor is wired to the in-flight connection via
    /// `update_connection` if a connection is in progress and its
    /// `from_node` matches one of the moved nodes. Parent expansion is
    /// handled by `handle_expand_parent` from `rgraph-core`.
    ///
    /// `drag_items` keys must be node ids that are currently in
    /// `node_lookup`; entries for unknown ids are silently skipped
    /// (TS does the same via `nodeLookup.get(id)?`).
    pub fn update_node_positions(
        &self,
        drag_items: &std::collections::HashMap<String, rgraph_core::types::nodes::NodeDragItem>,
        dragging: bool,
    ) {
        use rgraph_core::types::geometry::Position;
        use rgraph_core::utils::edges::positions::get_handle_position;
        use rgraph_core::utils::store::{handle_expand_parent, ParentExpandChild};

        let mut changes: Vec<NodeChange<N>> = Vec::with_capacity(drag_items.len());
        let mut parent_expand_children: Vec<ParentExpandChild> = Vec::new();

        // We need read access to `node_lookup` and `connection`,
        // and to issue `update_connection` if the cursor is dragging
        // the from-node of a live connection.
        let node_lookup = self.node_lookup.peek();
        let mut connection_snapshot = self.connection.peek().clone();
        let mut connection_dirty = false;

        for (id, drag_item) in drag_items {
            let Some(node) = node_lookup.get(id) else {
                continue;
            };

            let expand_parent =
                node.user.expand_parent.unwrap_or(false) && node.user.parent_id.is_some();

            let position = if expand_parent {
                XYPosition::new(drag_item.position.x.max(0.0), drag_item.position.y.max(0.0))
            } else {
                drag_item.position
            };

            changes.push(NodeChange::Position {
                id: id.clone(),
                position: Some(position),
                position_absolute: None,
                dragging: Some(dragging),
            });

            // Refresh the connection's `from` if we're moving its
            // from-node.
            if let ConnectionState::InProgress(in_progress) = &mut connection_snapshot
                && in_progress.from_node.user.id == node.user.id
            {
                let updated_from = get_handle_position(
                    node,
                    Some(&in_progress.from_handle),
                    Position::Left,
                    true,
                );
                in_progress.from = updated_from;
                connection_dirty = true;
            }

            if expand_parent
                && let Some(parent_id) = node.user.parent_id.clone()
            {
                parent_expand_children.push(ParentExpandChild {
                    id: id.clone(),
                    parent_id,
                    rect: rgraph_core::types::geometry::Rect {
                        x: drag_item.position_absolute.x,
                        y: drag_item.position_absolute.y,
                        width: drag_item.measured.width,
                        height: drag_item.measured.height,
                    },
                });
            }
        }

        if !parent_expand_children.is_empty() {
            let parent_lookup = self.parent_lookup.peek();
            let node_origin = *self.node_origin.peek();
            let parent_expand_changes =
                handle_expand_parent(&parent_expand_children, &node_lookup, &parent_lookup, node_origin);
            changes.extend(parent_expand_changes);
        }

        drop(node_lookup);

        // Apply per-mount middlewares (TS lines 258–260).
        let middlewares = self.on_nodes_change_middleware_map.peek().clone();
        let mut changes = changes;
        for mw in middlewares.values() {
            changes = mw(changes);
        }

        if connection_dirty {
            self.connection.clone().set(connection_snapshot);
        }

        self.trigger_node_changes(changes);
    }

    // ---------------------------------------------------------------------
    // `trigger_node_changes`, `trigger_edge_changes`
    // ---------------------------------------------------------------------

    /// Apply the supplied `NodeChange`s to the store. Mirrors TS
    /// `triggerNodeChanges` (lines 264–278).
    pub fn trigger_node_changes(&self, changes: Vec<NodeChange<N>>) {
        if changes.is_empty() {
            return;
        }
        let has_default_nodes = *self.has_default_nodes.peek();
        let debug = *self.debug.peek();
        if has_default_nodes {
            let current = self.nodes.peek().clone();
            let updated = apply_node_changes(changes.clone(), current);
            self.set_nodes(updated);
        }
        if debug {
            tracing::debug!(
                target: "rgraph::store",
                count = changes.len(),
                "trigger node changes"
            );
        }
        if let Some(handler) = *self.on_nodes_change.peek() {
            handler.call(changes);
        }
    }

    /// Apply the supplied `EdgeChange`s. Mirrors TS
    /// `triggerEdgeChanges` (lines 280–295).
    pub fn trigger_edge_changes(&self, changes: Vec<EdgeChange<E>>) {
        if changes.is_empty() {
            return;
        }
        let has_default_edges = *self.has_default_edges.peek();
        let debug = *self.debug.peek();
        if has_default_edges {
            let current = self.edges.peek().clone();
            let updated = apply_edge_changes(changes.clone(), current);
            self.set_edges(updated);
        }
        if debug {
            tracing::debug!(
                target: "rgraph::store",
                count = changes.len(),
                "trigger edge changes"
            );
        }
        if let Some(handler) = *self.on_edges_change.peek() {
            handler.call(changes);
        }
    }

    // ---------------------------------------------------------------------
    // Selection
    // ---------------------------------------------------------------------

    /// Add the listed node ids to the selection.
    ///
    /// Mirrors TS `addSelectedNodes` (lines 296–307).
    /// When `multiSelectionActive` is set, the change list only flips
    /// the supplied ids to `selected = true` (TS hack). Otherwise the
    /// full diff against the lookup is applied so previously-selected
    /// items are unselected.
    pub fn add_selected_nodes(&self, selected_node_ids: Vec<String>) {
        if *self.multi_selection_active.peek() {
            let node_changes: Vec<NodeChange<N>> = selected_node_ids
                .into_iter()
                .map(|id| create_node_selection_change(id, true))
                .collect();
            self.trigger_node_changes(node_changes);
            return;
        }

        let target: HashSet<String> = selected_node_ids.into_iter().collect();

        let node_changes = {
            let mut node_lookup = self.node_lookup.clone().write_unchecked();
            get_selection_changes_for_nodes(&mut node_lookup, &target)
        };
        let edge_changes = {
            let edge_lookup = self.edge_lookup.peek();
            get_selection_changes_for_edges(&edge_lookup, &HashSet::new())
        };

        self.trigger_node_changes(node_changes);
        self.trigger_edge_changes(edge_changes);
    }

    /// Add the listed edge ids to the selection. Mirrors TS
    /// `addSelectedEdges` (lines 308–319).
    pub fn add_selected_edges(&self, selected_edge_ids: Vec<String>) {
        if *self.multi_selection_active.peek() {
            let edge_changes: Vec<EdgeChange<E>> = selected_edge_ids
                .into_iter()
                .map(|id| create_edge_selection_change(id, true))
                .collect();
            self.trigger_edge_changes(edge_changes);
            return;
        }

        let target: HashSet<String> = selected_edge_ids.into_iter().collect();

        let edge_changes = {
            let edge_lookup = self.edge_lookup.peek();
            get_selection_changes_for_edges(&edge_lookup, &target)
        };
        let node_changes = {
            let mut node_lookup = self.node_lookup.clone().write_unchecked();
            get_selection_changes_for_nodes(&mut node_lookup, &HashSet::new())
        };

        self.trigger_edge_changes(edge_changes);
        self.trigger_node_changes(node_changes);
    }

    /// Unselect nodes and edges. Mirrors TS `unselectNodesAndEdges`
    /// (lines 320–357).
    pub fn unselect_nodes_and_edges(&self, params: UnselectNodesAndEdgesParams<N, E>) {
        let nodes_snapshot = self.nodes.peek().clone();
        let edges_snapshot = self.edges.peek().clone();
        let nodes_to_unselect = params.nodes.unwrap_or(nodes_snapshot);
        let edges_to_unselect = params.edges.unwrap_or(edges_snapshot);

        let mut node_changes: Vec<NodeChange<N>> = Vec::new();
        {
            let mut node_lookup = self.node_lookup.clone().write_unchecked();
            for node in &nodes_to_unselect {
                if !node.selected.unwrap_or(false) {
                    continue;
                }
                // Mutate the internal node so the next render sees it
                // unselected even before `on_nodes_change` fires (the
                // TS in-place hack from `getSelectionChanges`).
                if let Some(internal) = node_lookup.get_mut(&node.id) {
                    internal.user.selected = Some(false);
                }
                node_changes.push(create_node_selection_change(node.id.clone(), false));
            }
        }

        let mut edge_changes: Vec<EdgeChange<E>> = Vec::new();
        for edge in &edges_to_unselect {
            if !edge.selected.unwrap_or(false) {
                continue;
            }
            edge_changes.push(create_edge_selection_change(edge.id.clone(), false));
        }

        self.trigger_node_changes(node_changes);
        self.trigger_edge_changes(edge_changes);
    }

    /// Clear all selected nodes/edges, but only if
    /// `elements_selectable` is `true`. Mirrors TS
    /// `resetSelectedElements` (lines 375–393).
    pub fn reset_selected_elements(&self) {
        if !*self.elements_selectable.peek() {
            return;
        }

        let nodes_snapshot = self.nodes.peek().clone();
        let edges_snapshot = self.edges.peek().clone();

        let node_changes: Vec<NodeChange<N>> = nodes_snapshot
            .iter()
            .filter(|n| n.selected.unwrap_or(false))
            .map(|n| create_node_selection_change(n.id.clone(), false))
            .collect();
        let edge_changes: Vec<EdgeChange<E>> = edges_snapshot
            .iter()
            .filter(|e| e.selected.unwrap_or(false))
            .map(|e| create_edge_selection_change(e.id.clone(), false))
            .collect();

        self.trigger_node_changes(node_changes);
        self.trigger_edge_changes(edge_changes);
    }

    // ---------------------------------------------------------------------
    // Zoom / extent setters
    // ---------------------------------------------------------------------

    /// Update `min_zoom`, propagating to the `PanZoomInstance`. Mirrors
    /// TS `setMinZoom` (lines 358–363).
    pub fn set_min_zoom(&self, min_zoom: f64) {
        let max_zoom = *self.max_zoom.peek();
        if let Some(pan_zoom) = &*self.pan_zoom.peek() {
            pan_zoom.borrow_mut().set_scale_extent((min_zoom, max_zoom));
        }
        self.min_zoom.clone().set(min_zoom);
    }

    /// Update `max_zoom`, propagating to the `PanZoomInstance`. Mirrors
    /// TS `setMaxZoom` (lines 364–369).
    pub fn set_max_zoom(&self, max_zoom: f64) {
        let min_zoom = *self.min_zoom.peek();
        if let Some(pan_zoom) = &*self.pan_zoom.peek() {
            pan_zoom.borrow_mut().set_scale_extent((min_zoom, max_zoom));
        }
        self.max_zoom.clone().set(max_zoom);
    }

    /// Update `translate_extent`, propagating to the `PanZoomInstance`.
    /// Mirrors TS `setTranslateExtent` (lines 370–374).
    pub fn set_translate_extent(&self, translate_extent: CoordinateExtent) {
        if let Some(pan_zoom) = &*self.pan_zoom.peek() {
            pan_zoom.borrow_mut().set_translate_extent(translate_extent);
        }
        self.translate_extent.clone().set(translate_extent);
    }

    /// Update the node bounds, re-adopting all nodes so their absolute
    /// positions match. Mirrors TS `setNodeExtent` (lines 394–415).
    pub fn set_node_extent(&self, next_node_extent: CoordinateExtent) {
        let current = *self.node_extent.peek();
        if current == next_node_extent {
            return;
        }

        let options = AdoptUserNodesOptions {
            base: UpdateNodesOptions {
                node_origin: *self.node_origin.peek(),
                node_extent: next_node_extent,
                elevate_nodes_on_select: *self.elevate_nodes_on_select.peek(),
                z_index_mode: *self.z_index_mode.peek(),
            },
            check_equality: false,
        };

        let nodes = self.nodes.peek().clone();
        {
            let mut node_lookup = self.node_lookup.clone().write_unchecked();
            let mut parent_lookup = self.parent_lookup.clone().write_unchecked();
            adopt_user_nodes(&nodes, &mut node_lookup, &mut parent_lookup, &options);
        }
        self.node_extent.clone().set(next_node_extent);
    }

    // ---------------------------------------------------------------------
    // Pan / center
    // ---------------------------------------------------------------------

    /// Pan by a delta. Mirrors TS `panBy` (lines 416–420).
    ///
    /// Returns `true` once the underlying `PanZoomInstance` has applied
    /// the pan and the transform actually changed. In Phase 2 there is
    /// no `pan_zoom` yet, so this synchronously returns `false`. Full
    /// async wiring lands in Phase 4.
    pub fn pan_by(&self, delta: XYPosition) -> bool {
        let Some(pan_zoom) = self.pan_zoom.peek().clone() else {
            return false;
        };
        let transform = *self.transform.peek();
        let translate_extent = *self.translate_extent.peek();
        let width = *self.width.peek();
        let height = *self.height.peek();

        let promise = rgraph_core::utils::store::pan_by(
            delta,
            Some(&mut **pan_zoom.borrow_mut()),
            transform,
            translate_extent,
            width,
            height,
        );
        promise.block_take().unwrap_or(false)
    }

    /// Center on a flow-space position. Mirrors TS `setCenter`
    /// (lines 421–440). When no `pan_zoom` is available the method
    /// synchronously returns `false`.
    pub fn set_center(
        &self,
        x: f64,
        y: f64,
        options: Option<rgraph_core::types::viewport::SetCenterOptions>,
    ) -> bool {
        let Some(pan_zoom) = self.pan_zoom.peek().clone() else {
            return false;
        };
        let width = *self.width.peek();
        let height = *self.height.peek();
        let max_zoom = *self.max_zoom.peek();

        let next_zoom = options.as_ref().and_then(|o| o.zoom).unwrap_or(max_zoom);

        let viewport = Viewport {
            x: width / 2.0 - x * next_zoom,
            y: height / 2.0 - y * next_zoom,
            zoom: next_zoom,
        };

        let transform_options = options.map(|o| rgraph_core::types::panzoom::PanZoomTransformOptions {
            duration: o.base.duration,
            ease: o.base.ease,
            interpolate: o.base.interpolate,
        });

        let promise = pan_zoom.borrow_mut().set_viewport(viewport, transform_options);
        promise.block_take().unwrap_or(false)
    }

    // ---------------------------------------------------------------------
    // Connection
    // ---------------------------------------------------------------------

    /// Cancel the in-flight connection (TS `cancelConnection`, lines 441–445).
    pub fn cancel_connection(&self) {
        self.connection.clone().set(initial_connection());
    }

    /// Replace the in-flight connection state. Mirrors TS
    /// `updateConnection` (lines 446–448).
    pub fn update_connection(&self, connection: ConnectionState<InternalNode<N>>) {
        self.connection.clone().set(connection);
    }

    // ---------------------------------------------------------------------
    // Reset
    // ---------------------------------------------------------------------

    /// Drop every store field back to its initial value. Mirrors TS
    /// `reset` (line 450).
    ///
    /// The `InitialStateParams` are taken from the *current* zoom and
    /// extent settings so the host doesn't lose its prop-controlled
    /// configuration; the rest goes back to framework defaults.
    pub fn reset(&self) {
        let params = InitialStateParams::<N, E> {
            min_zoom: Some(*self.min_zoom.peek()),
            max_zoom: Some(*self.max_zoom.peek()),
            node_origin: Some(*self.node_origin.peek()),
            node_extent: Some(*self.node_extent.peek()),
            z_index_mode: Some(*self.z_index_mode.peek()),
            ..InitialStateParams::default()
        };
        let fresh = initial_state(params);
        self.write_state(fresh);
    }

    /// Overwrite every signal with the values from `state`. Used by
    /// [`Self::reset`] and by Phase 4's `resolve_fit_view`.
    pub fn write_state(&self, state: crate::types::store::RGraphStoreState<N, E>) {
        self.rf_id.clone().set(state.rf_id);
        self.width.clone().set(state.width);
        self.height.clone().set(state.height);
        self.transform.clone().set(state.transform);

        self.nodes.clone().set(state.nodes);
        self.nodes_initialized.clone().set(state.nodes_initialized);
        self.node_lookup.clone().set(state.node_lookup);
        self.parent_lookup.clone().set(state.parent_lookup);
        self.edges.clone().set(state.edges);
        self.edge_lookup.clone().set(state.edge_lookup);
        self.connection_lookup.clone().set(state.connection_lookup);

        self.on_nodes_change.clone().set(state.on_nodes_change);
        self.on_edges_change.clone().set(state.on_edges_change);
        self.has_default_nodes.clone().set(state.has_default_nodes);
        self.has_default_edges.clone().set(state.has_default_edges);

        self.dom_node_id.clone().set(state.dom_node_id);
        self.pane_dragging.clone().set(state.pane_dragging);
        self.no_pan_class_name.clone().set(state.no_pan_class_name);
        // pan_zoom is intentionally left as-is — restoring its slot is
        // managed by Phase 4's ZoomPane life-cycle.

        self.min_zoom.clone().set(state.min_zoom);
        self.max_zoom.clone().set(state.max_zoom);
        self.translate_extent.clone().set(state.translate_extent);
        self.node_extent.clone().set(state.node_extent);
        self.node_origin.clone().set(state.node_origin);
        self.node_drag_threshold.clone().set(state.node_drag_threshold);
        self.connection_drag_threshold
            .clone()
            .set(state.connection_drag_threshold);

        self.nodes_selection_active
            .clone()
            .set(state.nodes_selection_active);
        self.user_selection_active.clone().set(state.user_selection_active);
        self.user_selection_rect.clone().set(state.user_selection_rect);

        self.connection.clone().set(state.connection);
        self.connection_mode.clone().set(state.connection_mode);
        self.connection_click_start_handle
            .clone()
            .set(state.connection_click_start_handle);

        self.snap_to_grid.clone().set(state.snap_to_grid);
        self.snap_grid.clone().set(state.snap_grid);

        self.nodes_draggable.clone().set(state.nodes_draggable);
        self.auto_pan_on_node_focus.clone().set(state.auto_pan_on_node_focus);
        self.nodes_connectable.clone().set(state.nodes_connectable);
        self.nodes_focusable.clone().set(state.nodes_focusable);
        self.edges_focusable.clone().set(state.edges_focusable);
        self.edges_reconnectable.clone().set(state.edges_reconnectable);
        self.elements_selectable.clone().set(state.elements_selectable);
        self.elevate_nodes_on_select.clone().set(state.elevate_nodes_on_select);
        self.elevate_edges_on_select.clone().set(state.elevate_edges_on_select);
        self.select_nodes_on_drag.clone().set(state.select_nodes_on_drag);
        self.multi_selection_active.clone().set(state.multi_selection_active);

        self.on_node_drag_start.clone().set(state.on_node_drag_start);
        self.on_node_drag.clone().set(state.on_node_drag);
        self.on_node_drag_stop.clone().set(state.on_node_drag_stop);
        self.on_selection_drag_start.clone().set(state.on_selection_drag_start);
        self.on_selection_drag.clone().set(state.on_selection_drag);
        self.on_selection_drag_stop.clone().set(state.on_selection_drag_stop);
        self.on_move_start.clone().set(state.on_move_start);
        self.on_move.clone().set(state.on_move);
        self.on_move_end.clone().set(state.on_move_end);

        self.on_connect.clone().set(state.on_connect);
        self.on_connect_start.clone().set(state.on_connect_start);
        self.on_connect_end.clone().set(state.on_connect_end);
        self.on_click_connect_start.clone().set(state.on_click_connect_start);
        self.on_click_connect_end.clone().set(state.on_click_connect_end);

        self.connect_on_click.clone().set(state.connect_on_click);
        self.default_edge_options.clone().set(state.default_edge_options);

        self.fit_view_queued.clone().set(state.fit_view_queued);
        self.fit_view_options.clone().set(state.fit_view_options);
        // fit_view_resolver intentionally left alone.

        self.on_nodes_delete.clone().set(state.on_nodes_delete);
        self.on_edges_delete.clone().set(state.on_edges_delete);
        self.on_delete.clone().set(state.on_delete);
        self.on_error.clone().set(state.on_error);

        self.on_viewport_change_start.clone().set(state.on_viewport_change_start);
        self.on_viewport_change.clone().set(state.on_viewport_change);
        self.on_viewport_change_end.clone().set(state.on_viewport_change_end);
        self.on_before_delete.clone().set(state.on_before_delete);

        self.on_selection_change_handlers
            .clone()
            .set(state.on_selection_change_handlers);

        self.aria_live_message.clone().set(state.aria_live_message);
        self.auto_pan_on_connect.clone().set(state.auto_pan_on_connect);
        self.auto_pan_on_node_drag.clone().set(state.auto_pan_on_node_drag);
        self.auto_pan_speed.clone().set(state.auto_pan_speed);
        self.connection_radius.clone().set(state.connection_radius);

        self.is_valid_connection.clone().set(state.is_valid_connection);
        self.lib.clone().set(state.lib);
        self.debug.clone().set(state.debug);
        self.aria_label_config.clone().set(state.aria_label_config);
        self.z_index_mode.clone().set(state.z_index_mode);

        self.on_nodes_change_middleware_map
            .clone()
            .set(state.on_nodes_change_middleware_map);
        self.on_edges_change_middleware_map
            .clone()
            .set(state.on_edges_change_middleware_map);
    }
}

// ---------------------------------------------------------------------------
// Tiny helper to call `adopt_user_nodes` against the two `WritableRef`s
// without fighting the borrow checker by spelling them out at the call
// site each time.
// ---------------------------------------------------------------------------

trait NodeLookupWriteExt<D: Clone + PartialEq> {
    /// Borrow self plus the parent lookup mutably and adopt the
    /// supplied user nodes.
    fn with_combined(
        &mut self,
        parent_lookup: &mut ParentLookup<D>,
        nodes: &[Node<D>],
        options: &AdoptUserNodesOptions,
    ) -> (bool, bool);
}

impl<D: Clone + PartialEq> NodeLookupWriteExt<D>
    for rgraph_core::types::nodes::NodeLookup<D>
{
    fn with_combined(
        &mut self,
        parent_lookup: &mut ParentLookup<D>,
        nodes: &[Node<D>],
        options: &AdoptUserNodesOptions,
    ) -> (bool, bool) {
        let r = adopt_user_nodes(nodes, self, parent_lookup, options);
        (r.nodes_initialized, r.has_selected_nodes)
    }
}
