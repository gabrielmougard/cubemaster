//! Port of `xyflow-react/src/store/initialState.ts`.
//!
//! Status: Phase 2 — implemented.
//!
//! [`initial_state`] builds a fresh [`RGraphStoreState`] populated with
//! framework defaults plus the user-supplied initial nodes/edges. It
//! also pre-runs `adopt_user_nodes` and `update_connection_lookup` so
//! the lookups are populated on first render — matching what
//! `getInitialState` does in the TS source.

#![allow(clippy::module_name_repetitions)]

use rgraph_core::constants::{AriaLabelConfig, INFINITE_EXTENT};
use rgraph_core::types::connection::{initial_connection, ConnectionLookup, ConnectionMode};
use rgraph_core::types::edges::EdgeLookup;
use rgraph_core::types::geometry::{Transform, XYPosition};
use rgraph_core::types::nodes::{NodeLookup, NodeOrigin, ParentLookup};
use rgraph_core::types::viewport::{SnapGrid, ZIndexMode};
use rgraph_core::utils::graph::{get_internal_nodes_bounds, GetInternalNodesBoundsParams};
use rgraph_core::utils::general::{get_viewport_for_bounds};
use rgraph_core::utils::store::{adopt_user_nodes, update_connection_lookup, AdoptUserNodesOptions, UpdateNodesOptions};
use rgraph_core::types::viewport::Padding;

use crate::types::edges::Edge;
use crate::types::nodes::Node;
use crate::types::store::RGraphStoreState;
use crate::utils::general::PtrEq;

/// Parameter bundle for [`initial_state`]. Mirrors the destructured
/// argument shape of TS `getInitialState({ … })`.
///
/// Every field is optional. Defaults match the TS source:
/// * `min_zoom = 0.5`,
/// * `max_zoom = 2.0`,
/// * `z_index_mode = Basic`,
/// * `node_origin = [0, 0]`,
/// * `node_extent = INFINITE_EXTENT`.
pub struct InitialStateParams<N: Clone + PartialEq + 'static = (), E: Clone + 'static = ()> {
    /// Controlled nodes (TS `nodes`).
    pub nodes: Option<Vec<Node<N>>>,
    /// Controlled edges (TS `edges`).
    pub edges: Option<Vec<Edge<E>>>,
    /// Uncontrolled initial nodes (TS `defaultNodes`).
    pub default_nodes: Option<Vec<Node<N>>>,
    /// Uncontrolled initial edges (TS `defaultEdges`).
    pub default_edges: Option<Vec<Edge<E>>>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub fit_view: Option<bool>,
    pub fit_view_options: Option<PtrEq<crate::types::general::FitViewOptions>>,
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub node_origin: Option<NodeOrigin>,
    pub node_extent: Option<rgraph_core::types::geometry::CoordinateExtent>,
    pub z_index_mode: Option<ZIndexMode>,
}

// Manual `Default` because `#[derive(Default)]` would propagate
// `N: Default`/`E: Default` bounds, which is too restrictive — all
// fields here are `Option<…>` and default to `None` regardless of the
// inner type's `Default`-ness.
impl<N: Clone + PartialEq + 'static, E: Clone + 'static> Default for InitialStateParams<N, E> {
    fn default() -> Self {
        InitialStateParams {
            nodes: None,
            edges: None,
            default_nodes: None,
            default_edges: None,
            width: None,
            height: None,
            fit_view: None,
            fit_view_options: None,
            min_zoom: None,
            max_zoom: None,
            node_origin: None,
            node_extent: None,
            z_index_mode: None,
        }
    }
}

/// Build the data half of the store from `params`.
///
/// Mirrors `getInitialState` (TS lines 19–157). The TS resolves
/// `defaultNodes ?? nodes ?? []`; we do the same. After populating the
/// lookups we compute the initial `transform` when `fit_view = true`
/// (and `width`/`height` are known, which is rare on first render but
/// supported by the TS source for SSR use cases).
#[must_use]
pub fn initial_state<N: Clone + PartialEq + 'static, E: Clone + 'static>(
    params: InitialStateParams<N, E>,
) -> RGraphStoreState<N, E> {
    let min_zoom = params.min_zoom.unwrap_or(0.5);
    let max_zoom = params.max_zoom.unwrap_or(2.0);
    let node_origin = params.node_origin.unwrap_or((0.0, 0.0));
    let node_extent = params.node_extent.unwrap_or(INFINITE_EXTENT);
    let z_index_mode = params.z_index_mode.unwrap_or(ZIndexMode::Basic);
    let fit_view = params.fit_view.unwrap_or(false);

    let store_edges = params.default_edges.clone().or(params.edges.clone()).unwrap_or_default();
    let store_nodes = params.default_nodes.clone().or(params.nodes.clone()).unwrap_or_default();

    let mut node_lookup: NodeLookup<N> = NodeLookup::default();
    let mut parent_lookup: ParentLookup<N> = ParentLookup::default();
    let mut connection_lookup: ConnectionLookup = ConnectionLookup::default();
    let mut edge_lookup: EdgeLookup<E> = EdgeLookup::default();

    update_connection_lookup(&mut connection_lookup, &mut edge_lookup, &store_edges);

    let adopt_options = AdoptUserNodesOptions {
        base: UpdateNodesOptions {
            node_origin,
            node_extent,
            elevate_nodes_on_select: true,
            z_index_mode,
        },
        check_equality: true,
    };
    let adopt_result = adopt_user_nodes(&store_nodes, &mut node_lookup, &mut parent_lookup, &adopt_options);

    // Compute initial transform when fit_view is requested AND we know
    // the viewport size. On Dioxus desktop the viewport size is only
    // available after the first paint, so this normally falls through
    // to `Transform::IDENTITY`; the fit will then be reapplied once
    // `width`/`height` are populated by `useResizeHandler`.
    let mut transform = Transform::IDENTITY;
    if fit_view
        && let (Some(width), Some(height)) = (params.width, params.height)
    {
        let bounds = get_internal_nodes_bounds(
            &node_lookup,
            GetInternalNodesBoundsParams {
                filter: Some(Box::new(|n: &rgraph_core::types::nodes::InternalNode<N>| {
                    let has_w = n.measured.width.is_some() || n.user.width.is_some() || n.user.initial_width.is_some();
                    let has_h = n.measured.height.is_some() || n.user.height.is_some() || n.user.initial_height.is_some();
                    has_w && has_h
                })),
            },
        );

        // Pull `padding` from the fit-view options if supplied,
        // otherwise default to 10% (the TS `padding ?? 0.1`).
        let padding = params
            .fit_view_options
            .as_ref()
            .and_then(|opts| opts.padding)
            .unwrap_or_else(|| Padding::factor(0.1));

        let viewport = get_viewport_for_bounds(bounds, width, height, min_zoom, max_zoom, padding);
        transform = Transform::new(viewport.x, viewport.y, viewport.zoom);
    }

    RGraphStoreState {
        rf_id: "1".to_string(),
        width: params.width.unwrap_or(0.0),
        height: params.height.unwrap_or(0.0),
        transform,

        nodes: store_nodes,
        nodes_initialized: adopt_result.nodes_initialized,
        node_lookup,
        parent_lookup,
        edges: store_edges,
        edge_lookup,
        connection_lookup,

        on_nodes_change: None,
        on_edges_change: None,
        has_default_nodes: params.default_nodes.is_some(),
        has_default_edges: params.default_edges.is_some(),

        dom_node_id: None,
        pane_dragging: false,
        no_pan_class_name: "nopan".to_string(),
        pan_zoom: None,

        min_zoom,
        max_zoom,
        translate_extent: INFINITE_EXTENT,
        node_extent,
        node_origin,
        node_drag_threshold: 1.0,
        connection_drag_threshold: 1.0,

        nodes_selection_active: false,
        user_selection_active: false,
        user_selection_rect: None,

        connection: initial_connection(),
        connection_mode: ConnectionMode::Strict,
        connection_click_start_handle: None,

        snap_to_grid: false,
        snap_grid: SnapGrid::from((15.0, 15.0)),

        nodes_draggable: true,
        auto_pan_on_node_focus: true,
        nodes_connectable: true,
        nodes_focusable: true,
        edges_focusable: true,
        edges_reconnectable: true,
        elements_selectable: true,
        elevate_nodes_on_select: true,
        elevate_edges_on_select: false,
        select_nodes_on_drag: true,

        multi_selection_active: false,

        on_node_drag_start: None,
        on_node_drag: None,
        on_node_drag_stop: None,

        on_selection_drag_start: None,
        on_selection_drag: None,
        on_selection_drag_stop: None,

        on_move_start: None,
        on_move: None,
        on_move_end: None,

        on_connect: None,
        on_connect_start: None,
        on_connect_end: None,
        on_click_connect_start: None,
        on_click_connect_end: None,

        connect_on_click: true,
        default_edge_options: None,

        fit_view_queued: fit_view,
        fit_view_options: params.fit_view_options,
        fit_view_resolver: None,

        on_nodes_delete: None,
        on_edges_delete: None,
        on_delete: None,
        on_error: None,

        on_viewport_change_start: None,
        on_viewport_change: None,
        on_viewport_change_end: None,
        on_before_delete: None,

        on_selection_change_handlers: Vec::new(),

        aria_live_message: String::new(),
        auto_pan_on_connect: true,
        auto_pan_on_node_drag: true,
        auto_pan_speed: 15.0,
        connection_radius: 20.0,

        is_valid_connection: None,

        // The TS source sets `lib: 'react'`; we identify the Dioxus port.
        lib: "dioxus".to_string(),
        debug: false,
        aria_label_config: AriaLabelConfig::default(),

        z_index_mode,

        on_nodes_change_middleware_map: std::collections::HashMap::new(),
        on_edges_change_middleware_map: std::collections::HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_correct() {
        let s: RGraphStoreState = initial_state(InitialStateParams::default());
        assert_eq!(s.min_zoom, 0.5);
        assert_eq!(s.max_zoom, 2.0);
        assert_eq!(s.node_origin, (0.0, 0.0));
        assert_eq!(s.transform, Transform::IDENTITY);
        assert!(s.elements_selectable);
        assert!(!s.snap_to_grid);
        assert_eq!(s.lib, "dioxus");
        assert!(s.nodes.is_empty());
        assert!(!s.has_default_nodes);
        assert!(!s.fit_view_queued);
    }

    #[test]
    fn default_nodes_flag_is_set_when_provided() {
        let s: RGraphStoreState<(), ()> = initial_state(InitialStateParams {
            default_nodes: Some(vec![Node::<()>::minimal("n1", 0.0, 0.0)]),
            ..InitialStateParams::default()
        });
        assert!(s.has_default_nodes);
        assert_eq!(s.nodes.len(), 1);
    }

    #[test]
    fn nodes_take_precedence_over_default_nodes_only_when_default_is_none() {
        // `defaultNodes` wins when set (TS line 53).
        let s: RGraphStoreState<(), ()> = initial_state(InitialStateParams {
            default_nodes: Some(vec![Node::<()>::minimal("d", 0.0, 0.0)]),
            nodes: Some(vec![Node::<()>::minimal("n", 0.0, 0.0)]),
            ..InitialStateParams::default()
        });
        assert_eq!(s.nodes[0].id, "d");
    }

    #[test]
    fn nodes_initialized_reflects_adoption() {
        let s: RGraphStoreState<(), ()> = initial_state(InitialStateParams::default());
        // Empty list → not initialized.
        assert!(!s.nodes_initialized);

        // Node *without* measured dims → adoption returns `false` (the
        // store waits for ResizeObserver to populate dims).
        let s2: RGraphStoreState<(), ()> = initial_state(InitialStateParams {
            nodes: Some(vec![Node::<()>::minimal("n", 0.0, 0.0)]),
            ..InitialStateParams::default()
        });
        assert!(!s2.nodes_initialized);

        // Node *with* measured dims → adoption succeeds.
        let mut n = Node::<()>::minimal("n", 0.0, 0.0);
        n.measured = Some(rgraph_core::types::nodes::MeasuredDimensions {
            width: Some(10.0),
            height: Some(10.0),
        });
        let s3: RGraphStoreState<(), ()> = initial_state(InitialStateParams {
            nodes: Some(vec![n]),
            ..InitialStateParams::default()
        });
        assert!(s3.nodes_initialized);
    }

    #[test]
    fn fit_view_queued_mirrors_fit_view_flag() {
        let s: RGraphStoreState<(), ()> =
            initial_state(InitialStateParams { fit_view: Some(true), ..InitialStateParams::default() });
        assert!(s.fit_view_queued);
    }

    #[test]
    fn connection_lookup_populated_from_edges() {
        let edges = vec![
            Edge::<()>::minimal("e1", "a", "b"),
            Edge::<()>::minimal("e2", "a", "c"),
        ];
        let s: RGraphStoreState<(), ()> = initial_state(InitialStateParams {
            edges: Some(edges),
            ..InitialStateParams::default()
        });
        assert_eq!(s.edge_lookup.len(), 2);
        // The connection_lookup is keyed by node id, then by handle key.
        assert!(s.connection_lookup.contains_key("a"));
    }

    #[test]
    fn initial_transform_zero_when_no_size_for_fit_view() {
        // fit_view requested but no width/height → transform stays
        // identity; fit will reapply later.
        let s: RGraphStoreState<(), ()> = initial_state(InitialStateParams {
            nodes: Some(vec![Node::<()>::minimal("n", 0.0, 0.0)]),
            fit_view: Some(true),
            ..InitialStateParams::default()
        });
        assert_eq!(s.transform, Transform::IDENTITY);
        assert!(s.fit_view_queued);
    }

    #[test]
    fn initial_transform_computed_when_size_known() {
        let mut n = Node::<()>::minimal("n", 0.0, 0.0);
        n.initial_width = Some(100.0);
        n.initial_height = Some(100.0);
        let s: RGraphStoreState<(), ()> = initial_state(InitialStateParams {
            nodes: Some(vec![n]),
            fit_view: Some(true),
            width: Some(1000.0),
            height: Some(1000.0),
            ..InitialStateParams::default()
        });
        // Transform should be non-identity (some zoom + translation).
        assert!(s.transform != Transform::IDENTITY);
        assert!(s.transform.scale() > 0.0);
    }
}
