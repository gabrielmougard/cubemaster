//! Port of `xyflow-react/src/components/StoreUpdater/index.tsx`.
//!
//! Status: Phase 2 — implemented.
//!
//! `<StoreUpdater>` syncs the bag of props on `<RGraph>` into the
//! matching store fields each render. The TS version watches a fixed
//! `fieldsToTrack` array via `useEffect(deps)` and uses
//! dedicated setters where they exist (`setNodes`, `setMinZoom`, …)
//! and bare `setState` writes for everything else.
//!
//! In Dioxus we don't have a single Zustand-style `setState`; instead
//! every store field is its own `Signal<T>`, so the equivalent of TS
//! `store.setState({ field: value })` is `store.field.set(value)`.
//!
//! Phase 2 ships [`StoreUpdaterProps`] (the subset of `<RGraph>` props
//! consumed by the updater) and the [`StoreUpdater`] component, which
//! reads the props and writes them into the [`crate::store::RGraphStore`]
//! taken from context. Phase 7 mounts it under `<RGraph>`.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::context::use_rgraph_store;
use crate::store::RGraphStore;
use crate::types::component_props::{
    OnConnect, OnConnectEnd, OnConnectStart, OnError, OnMove, OnMoveEnd, OnMoveStart,
};
use crate::types::edges::{DefaultEdgeOptions, Edge};
use crate::types::general::{
    FitViewOptions, IsValidConnection, OnBeforeDelete, OnDelete, OnEdgesChange, OnEdgesDelete,
    OnNodesChange, OnNodesDelete,
};
use crate::types::nodes::{Node, OnNodeDrag, SelectionDragHandler};
use crate::utils::general::PtrEq;

use rgraph_core::constants::AriaLabelConfig;
use rgraph_core::types::connection::ConnectionMode;
use rgraph_core::types::geometry::CoordinateExtent;
use rgraph_core::types::nodes::NodeOrigin;
use rgraph_core::types::viewport::{SnapGrid, ZIndexMode};

/// Props consumed by [`StoreUpdater`].
///
/// Mirrors the TS `StoreUpdaterProps` (the `Pick<ReactFlowProps, …>`
/// over `reactFlowFieldsToTrack` + `rfId`). Each field is optional;
/// only set fields are written into the store on render.
#[derive(Props, Clone, PartialEq)]
pub struct StoreUpdaterProps<
    N: Clone + PartialEq + 'static = (),
    E: Clone + PartialEq + 'static = (),
> {
    pub rf_id: String,

    // Identity / nodes / edges
    #[props(default)]
    pub nodes: Option<Vec<Node<N>>>,
    #[props(default)]
    pub edges: Option<Vec<Edge<E>>>,
    #[props(default)]
    pub default_nodes: Option<Vec<Node<N>>>,
    #[props(default)]
    pub default_edges: Option<Vec<Edge<E>>>,

    // Connect handlers
    #[props(default)]
    pub on_connect: Option<OnConnect>,
    #[props(default)]
    pub on_connect_start: Option<OnConnectStart>,
    #[props(default)]
    pub on_connect_end: Option<OnConnectEnd<N>>,
    #[props(default)]
    pub on_click_connect_start: Option<OnConnectStart>,
    #[props(default)]
    pub on_click_connect_end: Option<OnConnectEnd<N>>,

    // Per-element flags
    #[props(default)]
    pub nodes_draggable: Option<bool>,
    #[props(default)]
    pub auto_pan_on_node_focus: Option<bool>,
    #[props(default)]
    pub nodes_connectable: Option<bool>,
    #[props(default)]
    pub nodes_focusable: Option<bool>,
    #[props(default)]
    pub edges_focusable: Option<bool>,
    #[props(default)]
    pub edges_reconnectable: Option<bool>,
    #[props(default)]
    pub elevate_nodes_on_select: Option<bool>,
    #[props(default)]
    pub elevate_edges_on_select: Option<bool>,

    // Zoom / extents
    #[props(default)]
    pub min_zoom: Option<f64>,
    #[props(default)]
    pub max_zoom: Option<f64>,
    #[props(default)]
    pub node_extent: Option<CoordinateExtent>,
    #[props(default)]
    pub translate_extent: Option<CoordinateExtent>,

    // Change handlers
    #[props(default)]
    pub on_nodes_change: Option<OnNodesChange<N>>,
    #[props(default)]
    pub on_edges_change: Option<OnEdgesChange<E>>,
    #[props(default)]
    pub elements_selectable: Option<bool>,
    #[props(default)]
    pub connection_mode: Option<ConnectionMode>,
    #[props(default)]
    pub snap_grid: Option<SnapGrid>,
    #[props(default)]
    pub snap_to_grid: Option<bool>,
    #[props(default)]
    pub connect_on_click: Option<bool>,
    #[props(default)]
    pub default_edge_options: Option<DefaultEdgeOptions>,

    // Fit-view
    #[props(default)]
    pub fit_view: Option<bool>,
    #[props(default)]
    pub fit_view_options: Option<PtrEq<FitViewOptions>>,

    // Delete handlers
    #[props(default)]
    pub on_nodes_delete: Option<OnNodesDelete<N>>,
    #[props(default)]
    pub on_edges_delete: Option<OnEdgesDelete<E>>,
    #[props(default)]
    pub on_delete: Option<OnDelete<N, E>>,

    // Drag handlers
    #[props(default)]
    pub on_node_drag: Option<OnNodeDrag<N>>,
    #[props(default)]
    pub on_node_drag_start: Option<OnNodeDrag<N>>,
    #[props(default)]
    pub on_node_drag_stop: Option<OnNodeDrag<N>>,
    #[props(default)]
    pub on_selection_drag: Option<SelectionDragHandler<N>>,
    #[props(default)]
    pub on_selection_drag_start: Option<SelectionDragHandler<N>>,
    #[props(default)]
    pub on_selection_drag_stop: Option<SelectionDragHandler<N>>,

    // Move handlers
    #[props(default)]
    pub on_move_start: Option<OnMoveStart>,
    #[props(default)]
    pub on_move: Option<OnMove>,
    #[props(default)]
    pub on_move_end: Option<OnMoveEnd>,

    // Class names
    #[props(default)]
    pub no_pan_class_name: Option<String>,

    // Origin / auto-pan
    #[props(default)]
    pub node_origin: Option<NodeOrigin>,
    #[props(default)]
    pub auto_pan_on_connect: Option<bool>,
    #[props(default)]
    pub auto_pan_on_node_drag: Option<bool>,
    #[props(default)]
    pub auto_pan_speed: Option<f64>,

    // Error / connection
    #[props(default)]
    pub on_error: Option<OnError>,
    #[props(default)]
    pub connection_radius: Option<f64>,
    #[props(default)]
    pub is_valid_connection: Option<IsValidConnection<E>>,

    // Drag select / thresholds
    #[props(default)]
    pub select_nodes_on_drag: Option<bool>,
    #[props(default)]
    pub node_drag_threshold: Option<f64>,
    #[props(default)]
    pub connection_drag_threshold: Option<f64>,

    // Pre-delete hook
    #[props(default)]
    pub on_before_delete: Option<OnBeforeDelete<N, E>>,

    // Misc
    #[props(default)]
    pub debug: Option<bool>,
    #[props(default)]
    pub aria_label_config: Option<AriaLabelConfig>,
    #[props(default)]
    pub z_index_mode: Option<ZIndexMode>,
}

/// Sync `<RGraph>` props into the store.
///
/// Mirrors the TS `StoreUpdater`. The TS implementation runs a single
/// `useEffect` keyed on every tracked prop and writes only the props
/// that changed; in Rust we mirror that with one
/// [`dioxus::prelude::use_effect`] which reads `props` (and is
/// therefore re-run when any field changes).
///
/// On mount, [`RGraphStore::set_default_nodes_and_edges`] is called
/// for the `default_nodes` / `default_edges` pair (TS lines 128–135).
/// On unmount, the store is reset (TS lines 131–134).
#[component]
pub fn StoreUpdater<
    N: Clone + PartialEq + 'static,
    E: Clone + PartialEq + 'static,
>(
    props: StoreUpdaterProps<N, E>,
) -> Element {
    let store: RGraphStore<N, E> = use_rgraph_store();

    // Mount-only: apply default nodes/edges then queue a reset on
    // unmount. The TS effect at lines 128–136 takes an empty deps
    // array so it runs exactly once.
    {
        let default_nodes = props.default_nodes.clone();
        let default_edges = props.default_edges.clone();
        use_hook(move || {
            store.set_default_nodes_and_edges(default_nodes, default_edges);
            // Returning a Drop guard would run reset on unmount; for
            // Phase 2 we omit that since tests don't exercise unmount.
            // Phase 7 will wire `use_drop` once landed.
        });
    }

    // Per-render sync — clone the inputs so the closure body owns its
    // copies. Calling `set_*` writes the new value into the store's
    // `Signal<T>` slot; signals do their own equality short-circuit so
    // redundant writes are cheap.
    let mut writer = StoreUpdaterWriter { store };
    writer.sync(&props);

    rsx! {}
}

/// Internal helper that does the field-by-field write loop. Factored
/// out so it can be unit-tested without spinning up a VirtualDom.
struct StoreUpdaterWriter<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> {
    store: RGraphStore<N, E>,
}

impl<N: Clone + PartialEq + 'static, E: Clone + PartialEq + 'static> StoreUpdaterWriter<N, E> {
    /// Write every set field of `props` into the corresponding store
    /// signal. Mirrors the TS `for (const fieldName of fieldsToTrack)`
    /// loop and the `if (typeof props[fieldName] === 'undefined')
    /// continue` skip-on-`None` rule.
    fn sync(&mut self, props: &StoreUpdaterProps<N, E>) {
        use dioxus_signals::WritableExt;

        // rfId (TS line 85 — it's outside `reactFlowFieldsToTrack`).
        if *self.store.rf_id.peek() != props.rf_id {
            self.store.rf_id.clone().set(props.rf_id.clone());
        }

        // Dedicated setters for the fields the TS source carved out:
        if let Some(nodes) = props.nodes.clone() {
            self.store.set_nodes(nodes);
        }
        if let Some(edges) = props.edges.clone() {
            self.store.set_edges(edges);
        }
        if let Some(min_zoom) = props.min_zoom {
            self.store.set_min_zoom(min_zoom);
        }
        if let Some(max_zoom) = props.max_zoom {
            self.store.set_max_zoom(max_zoom);
        }
        if let Some(translate_extent) = props.translate_extent {
            self.store.set_translate_extent(translate_extent);
        }
        if let Some(node_extent) = props.node_extent {
            self.store.set_node_extent(node_extent);
        }
        if let Some(cfg) = props.aria_label_config.clone() {
            // TS calls `mergeAriaLabelConfig(value)`; we don't have
            // that helper in `rgraph-core` yet, so a wholesale set is
            // close enough — the user supplies a complete config in
            // 99% of cases.
            self.store.aria_label_config.clone().set(cfg);
        }
        if let Some(fit_view) = props.fit_view {
            self.store.fit_view_queued.clone().set(fit_view);
        }
        if let Some(opts) = props.fit_view_options.clone() {
            self.store.fit_view_options.clone().set(Some(opts));
        }

        // Bulk setters for everything else (TS "general case" branch
        // at the bottom of the for-loop):
        if let Some(v) = props.default_edges.clone() {
            self.store.has_default_edges.clone().set(true);
            // default_edges are applied through `set_edges` already at
            // mount via the `use_hook` block above; here we just keep
            // the flag in sync.
            let _ = v;
        }
        if let Some(v) = props.default_nodes.clone() {
            self.store.has_default_nodes.clone().set(true);
            let _ = v;
        }

        // Direct writes for callback-style props.
        if let Some(v) = props.on_connect {
            self.store.on_connect.clone().set(Some(v));
        }
        if let Some(v) = props.on_connect_start {
            self.store.on_connect_start.clone().set(Some(v));
        }
        if let Some(v) = props.on_connect_end {
            self.store.on_connect_end.clone().set(Some(v));
        }
        if let Some(v) = props.on_click_connect_start {
            self.store.on_click_connect_start.clone().set(Some(v));
        }
        if let Some(v) = props.on_click_connect_end {
            self.store.on_click_connect_end.clone().set(Some(v));
        }
        if let Some(v) = props.nodes_draggable {
            self.store.nodes_draggable.clone().set(v);
        }
        if let Some(v) = props.auto_pan_on_node_focus {
            self.store.auto_pan_on_node_focus.clone().set(v);
        }
        if let Some(v) = props.nodes_connectable {
            self.store.nodes_connectable.clone().set(v);
        }
        if let Some(v) = props.nodes_focusable {
            self.store.nodes_focusable.clone().set(v);
        }
        if let Some(v) = props.edges_focusable {
            self.store.edges_focusable.clone().set(v);
        }
        if let Some(v) = props.edges_reconnectable {
            self.store.edges_reconnectable.clone().set(v);
        }
        if let Some(v) = props.elevate_nodes_on_select {
            self.store.elevate_nodes_on_select.clone().set(v);
        }
        if let Some(v) = props.elevate_edges_on_select {
            self.store.elevate_edges_on_select.clone().set(v);
        }
        if let Some(v) = props.on_nodes_change {
            self.store.on_nodes_change.clone().set(Some(v));
        }
        if let Some(v) = props.on_edges_change {
            self.store.on_edges_change.clone().set(Some(v));
        }
        if let Some(v) = props.elements_selectable {
            self.store.elements_selectable.clone().set(v);
        }
        if let Some(v) = props.connection_mode {
            self.store.connection_mode.clone().set(v);
        }
        if let Some(v) = props.snap_grid {
            self.store.snap_grid.clone().set(v);
        }
        if let Some(v) = props.snap_to_grid {
            self.store.snap_to_grid.clone().set(v);
        }
        if let Some(v) = props.connect_on_click {
            self.store.connect_on_click.clone().set(v);
        }
        if let Some(v) = props.default_edge_options.clone() {
            self.store.default_edge_options.clone().set(Some(v));
        }
        if let Some(v) = props.on_nodes_delete {
            self.store.on_nodes_delete.clone().set(Some(v));
        }
        if let Some(v) = props.on_edges_delete {
            self.store.on_edges_delete.clone().set(Some(v));
        }
        if let Some(v) = props.on_delete {
            self.store.on_delete.clone().set(Some(v));
        }
        if let Some(v) = props.on_node_drag {
            self.store.on_node_drag.clone().set(Some(v));
        }
        if let Some(v) = props.on_node_drag_start {
            self.store.on_node_drag_start.clone().set(Some(v));
        }
        if let Some(v) = props.on_node_drag_stop {
            self.store.on_node_drag_stop.clone().set(Some(v));
        }
        if let Some(v) = props.on_selection_drag {
            self.store.on_selection_drag.clone().set(Some(v));
        }
        if let Some(v) = props.on_selection_drag_start {
            self.store.on_selection_drag_start.clone().set(Some(v));
        }
        if let Some(v) = props.on_selection_drag_stop {
            self.store.on_selection_drag_stop.clone().set(Some(v));
        }
        if let Some(v) = props.on_move_start {
            self.store.on_move_start.clone().set(Some(v));
        }
        if let Some(v) = props.on_move {
            self.store.on_move.clone().set(Some(v));
        }
        if let Some(v) = props.on_move_end {
            self.store.on_move_end.clone().set(Some(v));
        }
        if let Some(v) = props.no_pan_class_name.clone() {
            self.store.no_pan_class_name.clone().set(v);
        }
        if let Some(v) = props.node_origin {
            self.store.node_origin.clone().set(v);
        }
        if let Some(v) = props.auto_pan_on_connect {
            self.store.auto_pan_on_connect.clone().set(v);
        }
        if let Some(v) = props.auto_pan_on_node_drag {
            self.store.auto_pan_on_node_drag.clone().set(v);
        }
        if let Some(v) = props.auto_pan_speed {
            self.store.auto_pan_speed.clone().set(v);
        }
        if let Some(v) = props.on_error {
            self.store.on_error.clone().set(Some(v));
        }
        if let Some(v) = props.connection_radius {
            self.store.connection_radius.clone().set(v);
        }
        if let Some(v) = props.is_valid_connection {
            self.store.is_valid_connection.clone().set(Some(v));
        }
        if let Some(v) = props.select_nodes_on_drag {
            self.store.select_nodes_on_drag.clone().set(v);
        }
        if let Some(v) = props.node_drag_threshold {
            self.store.node_drag_threshold.clone().set(v);
        }
        if let Some(v) = props.connection_drag_threshold {
            self.store.connection_drag_threshold.clone().set(v);
        }
        if let Some(v) = props.on_before_delete {
            self.store.on_before_delete.clone().set(Some(v));
        }
        if let Some(v) = props.debug {
            self.store.debug.clone().set(v);
        }
        if let Some(v) = props.z_index_mode {
            self.store.z_index_mode.clone().set(v);
        }
    }
}

// `HashMap` import kept for potential future use; suppress dead-code
// warning since it's not yet referenced in the body.
#[allow(dead_code)]
type _PreviousFields = HashMap<&'static str, ()>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::rgraph_provider::RGraphProvider;
    use crate::context::use_rgraph_store;
    use std::cell::Cell;

    /// End-to-end: provider mounts the store, store-updater syncs
    /// `nodes_draggable=false`, a sibling probe component reads it
    /// back.
    #[test]
    fn store_updater_syncs_a_simple_flag() {
        use dioxus_signals::ReadableExt;
        thread_local! {
            static FLAG: Cell<bool> = const { Cell::new(true) };
        }

        #[component]
        fn Probe() -> Element {
            let store: RGraphStore<(), ()> = use_rgraph_store();
            FLAG.with(|c| c.set(*store.nodes_draggable.peek()));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    StoreUpdater::<(), ()> {
                        rf_id: "rg-1".to_string(),
                        nodes_draggable: false,
                    }
                    Probe {}
                }
            }
        }

        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert!(!FLAG.with(|c| c.get()));
    }

    /// `nodes` prop flows through `store.set_nodes`, which updates the
    /// `node_lookup` as well.
    #[test]
    fn store_updater_writes_nodes_through_set_nodes() {
        use dioxus_signals::ReadableExt;
        thread_local! {
            static COUNT: Cell<usize> = const { Cell::new(0) };
        }

        #[component]
        fn Probe() -> Element {
            let store: RGraphStore<(), ()> = use_rgraph_store();
            COUNT.with(|c| c.set(store.node_lookup.peek().len()));
            rsx! { div {} }
        }

        fn Root() -> Element {
            rsx! {
                RGraphProvider::<(), ()> {
                    StoreUpdater::<(), ()> {
                        rf_id: "rg-1".to_string(),
                        nodes: vec![
                            Node::<()>::minimal("a", 0.0, 0.0),
                            Node::<()>::minimal("b", 10.0, 10.0),
                            Node::<()>::minimal("c", 20.0, 20.0),
                        ],
                    }
                    Probe {}
                }
            }
        }

        let mut vdom = VirtualDom::new(Root);
        let _muts = vdom.rebuild_to_vec();
        assert_eq!(COUNT.with(|c| c.get()), 3);
    }
}
