//! Port of `xyflow-react/src/types/general.ts`.
//!
//! Status: Phase 1 — implemented.
//!
//! Provides:
//!
//! * `OnNodesChange`, `OnEdgesChange`, `OnNodesDelete`, `OnEdgesDelete`,
//!   `OnDelete`, `OnBeforeDelete`, `IsValidConnection` — callback aliases.
//! * `NodeTypes` / `EdgeTypes` — registries mapping a string key to the
//!   Dioxus component responsible for rendering that node / edge variant.
//! * `UnselectNodesAndEdgesParams`, `OnSelectionChangeParams`, …
//! * `FitViewParams`, `FitViewOptions`, `FitView` — fit-view ergonomics.
//! * `OnInit` — convenience alias for the `on_init` callback.
//! * `ViewportHelperFunctions` — the trait-shape mirroring the TS
//!   interface used by the imperative instance.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dioxus::prelude::{Callback, Element};

use rgraph_core::types::connection::Connection;
use rgraph_core::types::changes::{EdgeChange, NodeChange};
use rgraph_core::types::geometry::XYPosition;
use rgraph_core::types::viewport::{
    FitViewOptionsBase, SnapGrid, Viewport, ViewportHelperFunctionOptions,
};

use crate::types::edges::{Edge, EdgeProps};
use crate::types::nodes::{Node, NodeProps};

// ---------------------------------------------------------------------------
// Per-element change handlers.
// ---------------------------------------------------------------------------

/// Callback fired by the store after node changes are applied (TS
/// `OnNodesChange<NodeType>`).
pub type OnNodesChange<D = ()> = Callback<Vec<NodeChange<D>>>;

/// Callback fired by the store after edge changes are applied (TS
/// `OnEdgesChange<EdgeType>`).
pub type OnEdgesChange<D = ()> = Callback<Vec<EdgeChange<D>>>;

/// `on_nodes_delete` — fires with the nodes that were just removed.
pub type OnNodesDelete<D = ()> = Callback<Vec<Node<D>>>;

/// `on_edges_delete` — fires with the edges that were just removed.
pub type OnEdgesDelete<D = ()> = Callback<Vec<Edge<D>>>;

/// Combined delete callback (`on_delete`). TS pairs the two lists into a
/// `{ nodes, edges }` object; we expose them as a tuple-like struct.
pub type OnDelete<N = (), E = ()> = Callback<OnDeleteArgs<N, E>>;

#[derive(Debug, Clone, PartialEq)]
pub struct OnDeleteArgs<N: Clone = (), E: Clone = ()> {
    pub nodes: Vec<Node<N>>,
    pub edges: Vec<Edge<E>>,
}

/// Pre-delete hook (`on_before_delete`). Returns the optionally
/// filtered set of nodes/edges that should be removed. TS uses
/// `OnBeforeDeleteBase<NodeType, EdgeType>` which is an async hook.
///
/// In the Rust port we keep the same signature; the boxed future is
/// produced by `Callback::call` returning a `Promise<Option<…>>`.
/// Phase 2's store will wire the async resolution.
pub type OnBeforeDelete<N = (), E = ()> = Callback<OnDeleteArgs<N, E>, Option<OnDeleteArgs<N, E>>>;

// ---------------------------------------------------------------------------
// Component-type registries.
// ---------------------------------------------------------------------------

/// A renderer-component for a custom node variant.
///
/// The TS type is `ComponentType<NodeProps & { data: any; type: any }>`.
/// In Dioxus we model it as a type-erased `Callback<NodeProps<serde_json::Value>, Element>`,
/// so a registry can hold renderers for many heterogeneous custom node
/// types. For phase 1 we expose an opaque `NodeRendererFn` alias and
/// leave the JSON-erased data type to phase 5 when actual node
/// components land — for now downstream code uses the generic
/// [`NodeRenderer<D>`] flavour.
///
/// `D: PartialEq` is required because [`NodeProps<D>`] derives
/// `Props` (a Dioxus props bag must be comparable for
/// memoisation).
pub type NodeRenderer<D = ()> = Callback<NodeProps<D>, Element>;

/// `NodeTypes<D>` — string-keyed registry of node renderers, mirroring
/// the TS `NodeTypes`. Built-in keys are `"input"`, `"output"`,
/// `"default"`, `"group"` (registered by the framework).
pub type NodeTypes<D = ()> = HashMap<String, NodeRenderer<D>>;

/// A renderer-component for a custom edge variant.
pub type EdgeRenderer<D = (), PathOptions = ()> =
    Callback<EdgeProps<D, PathOptions>, Element>;

/// `EdgeTypes<D, P>` — string-keyed registry of edge renderers. Built-in
/// keys are `"default"` (bezier), `"smoothstep"`, `"step"`, `"straight"`,
/// `"simplebezier"` (registered by the framework).
///
/// `PathOptions` defaults to `()` so the common case (custom edges
/// without path options) uses a one-parameter alias.
pub type EdgeTypes<D = (), PathOptions = ()> = HashMap<String, EdgeRenderer<D, PathOptions>>;

// ---------------------------------------------------------------------------
// Selection-related parameter bundles.
// ---------------------------------------------------------------------------

/// Argument bundle for the `unselect_nodes_and_edges` action.
///
/// Mirrors the TS `UnselectNodesAndEdgesParams`. Both fields are
/// optional — `None` means "all nodes" / "all edges".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UnselectNodesAndEdgesParams<N: Clone = (), E: Clone = ()> {
    pub nodes: Option<Vec<Node<N>>>,
    pub edges: Option<Vec<Edge<E>>>,
}

/// Payload of the `on_selection_change` callback.
///
/// Mirrors the TS `OnSelectionChangeParams`.
#[derive(Debug, Clone, PartialEq)]
pub struct OnSelectionChangeParams<N: Clone = (), E: Clone = ()> {
    pub nodes: Vec<Node<N>>,
    pub edges: Vec<Edge<E>>,
}

/// Callable signature of `on_selection_change`.
///
/// Mirrors the TS `OnSelectionChangeFunc`.
pub type OnSelectionChangeFunc<N = (), E = ()> = Callback<OnSelectionChangeParams<N, E>>;

// ---------------------------------------------------------------------------
// Fit-view parameter / option bundles.
// ---------------------------------------------------------------------------

/// Parameters passed to the internal `fit_view` routine.
///
/// Mirrors the TS `FitViewParams<NodeType> = FitViewParamsBase<NodeType>`.
/// Re-export of [`rgraph_core::utils::graph::FitViewportParams`] (TS
/// `FitViewParamsBase`).
pub use rgraph_core::utils::graph::FitViewportParams as FitViewParams;

/// Public options accepted by `fit_view(options)`.
///
/// Mirrors the TS `FitViewOptions<NodeType>`. Re-exports the
/// rgraph-core type unchanged.
pub type FitViewOptions = FitViewOptionsBase;

/// Convenience callback type for the imperative `fit_view` function on
/// the [`crate::types::instance::RGraphInstance`].
///
/// In the TS port this is `(options?: FitViewOptions) => Promise<boolean>`.
/// We use `Callback<Option<FitViewOptions>, bool>` here; the store
/// resolves the underlying promise synchronously (phase 4).
pub type FitView = Callback<Option<FitViewOptions>, bool>;

// ---------------------------------------------------------------------------
// OnInit.
// ---------------------------------------------------------------------------

/// Fired exactly once after the viewport is mounted and the first
/// render is committed, with a usable [`crate::types::instance::RGraphInstance`].
///
/// Mirrors the TS `OnInit<NodeType, EdgeType>`.
///
/// `Instance` is generic because [`crate::types::instance::RGraphInstance`]
/// is generic over node/edge data; concretely it is
/// `RGraphInstance<NodeData, EdgeData>`.
pub type OnInit<Instance> = Callback<Instance>;

// ---------------------------------------------------------------------------
// ViewportHelperFunctions.
// ---------------------------------------------------------------------------

/// Bundle of imperative helpers exposed through `use_rgraph()`.
///
/// Mirrors the TS `ViewportHelperFunctions` interface. In Rust we model
/// it as a struct of `Callback` values (rather than a trait) so the
/// store can hand out a single cloneable instance.
#[derive(Clone, PartialEq)]
pub struct ViewportHelperFunctions {
    /// Zoom in by 1.2× (TS `zoomIn`).
    pub zoom_in: Callback<Option<ViewportHelperFunctionOptions>>,
    /// Zoom out by 1/1.2× (TS `zoomOut`).
    pub zoom_out: Callback<Option<ViewportHelperFunctionOptions>>,
    /// Zoom to an absolute zoom level (TS `zoomTo(level, options?)`).
    pub zoom_to: Callback<ZoomToArgs>,
    /// Read the current zoom level.
    pub get_zoom: Callback<(), f64>,
    /// Set the viewport (TS `setViewport(viewport, options?)`).
    pub set_viewport: Callback<SetViewportArgs>,
    /// Read the current viewport.
    pub get_viewport: Callback<(), Viewport>,
    /// Center on a flow-space position (TS `setCenter(x, y, options?)`).
    pub set_center: Callback<SetCenterArgs>,
    /// Fit the viewport to a rect (TS `fitBounds(bounds, options?)`).
    pub fit_bounds: Callback<FitBoundsArgs>,
    /// Translate a screen-space point to flow-space.
    pub screen_to_flow_position: Callback<ScreenToFlowArgs, XYPosition>,
    /// Translate a flow-space point to screen-space.
    pub flow_to_screen_position: Callback<XYPosition, XYPosition>,
}

/// Args for `ViewportHelperFunctions::zoom_to`.
///
/// `options` contains an [`rgraph_core::types::viewport::EaseFn`] which
/// is not `Clone`, so this struct intentionally doesn't derive `Clone`
/// nor `PartialEq` — those bounds are unnecessary because the value is
/// constructed at the call site and passed straight into a callback.
#[derive(Debug)]
pub struct ZoomToArgs {
    pub zoom_level: f64,
    pub options: Option<ViewportHelperFunctionOptions>,
}

/// Args for `ViewportHelperFunctions::set_viewport`. See [`ZoomToArgs`]
/// for the trait-bound rationale.
#[derive(Debug)]
pub struct SetViewportArgs {
    pub viewport: Viewport,
    pub options: Option<ViewportHelperFunctionOptions>,
}

/// Args for `ViewportHelperFunctions::set_center`. See [`ZoomToArgs`]
/// for the trait-bound rationale.
#[derive(Debug)]
pub struct SetCenterArgs {
    pub x: f64,
    pub y: f64,
    pub options: Option<rgraph_core::types::viewport::SetCenterOptions>,
}

/// Args for `ViewportHelperFunctions::fit_bounds`. See [`ZoomToArgs`]
/// for the trait-bound rationale.
#[derive(Debug)]
pub struct FitBoundsArgs {
    pub bounds: rgraph_core::types::geometry::Rect,
    pub options: Option<rgraph_core::types::viewport::FitBoundsOptions>,
}

/// Args for `ViewportHelperFunctions::screen_to_flow_position`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScreenToFlowArgs {
    pub client_position: XYPosition,
    pub snap_to_grid: Option<bool>,
    pub snap_grid: Option<SnapGrid>,
}

// ---------------------------------------------------------------------------
// IsValidConnection.
// ---------------------------------------------------------------------------

/// Either a partial [`Connection`] or a fully-formed [`Edge`], accepted
/// by the user-supplied validator (TS `EdgeType | Connection`).
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionOrEdge<D: Clone = ()> {
    Connection(Connection),
    Edge(Edge<D>),
}

impl<D: Clone> From<Connection> for ConnectionOrEdge<D> {
    fn from(c: Connection) -> Self {
        ConnectionOrEdge::Connection(c)
    }
}

impl<D: Clone> From<Edge<D>> for ConnectionOrEdge<D> {
    fn from(e: Edge<D>) -> Self {
        ConnectionOrEdge::Edge(e)
    }
}

/// User-supplied validator that decides whether an edge / candidate
/// connection is allowed. Mirrors the TS `IsValidConnection<EdgeType>`.
pub type IsValidConnection<D = ()> = Callback<ConnectionOrEdge<D>, bool>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unselect_params_default_is_empty() {
        let p: UnselectNodesAndEdgesParams = UnselectNodesAndEdgesParams::default();
        assert!(p.nodes.is_none());
        assert!(p.edges.is_none());
    }

    #[test]
    fn connection_or_edge_from_works() {
        let c = Connection {
            source: "a".into(),
            target: "b".into(),
            source_handle: None,
            target_handle: None,
        };
        let _ce: ConnectionOrEdge = c.into();
    }
}
