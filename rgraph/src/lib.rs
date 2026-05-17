//! `rgraph` — Dioxus port of [`@xyflow/react`](https://github.com/xyflow/xyflow/tree/main/packages/react).
//!
//! Built on top of [`rgraph_core`] (the framework-agnostic Rust port of
//! `@xyflow/system`). This crate provides Dioxus components, hooks and a
//! reactive store mirroring the public surface of `@xyflow/react`.
//!
//! ## Status
//!
//! Phase 0 — **skeleton only**. Every module below is a stub annotated
//! with `TODO(rgraph/phaseN)` markers and the TypeScript file it ports.
//! See `README.md` for the full porting roadmap.
//!
//! ## Module overview
//!
//! * [`types`]      — pure data types (mirror of `xyflow-react/src/types/`).
//! * [`utils`]      — pure functions (`apply_node_changes`, `is_node`, …).
//! * [`store`]      — the reactive `RGraphStore` (signal-backed).
//! * [`context`]    — Dioxus context wrapper for the store.
//! * [`contexts`]   — direct ports of `xyflow-react/src/contexts/*.ts`.
//! * [`hooks`]      — Dioxus equivalents of every `useXxx.ts`.
//! * [`components`] — Dioxus equivalents of `xyflow-react/src/components/*`.
//! * [`container`]  — Dioxus equivalents of `xyflow-react/src/container/*`.
//! * [`dom`]        — DOM bridges (ResizeObserver, getBoundingClientRect, …)
//!                    that use the Dioxus desktop webview.
//! * [`additional_components`] — `Background`, `Controls`, `MiniMap`, …
//! * [`styles`]     — exposes the bundled CSS as `&'static str` constants.
//! * [`prelude`]    — `use rgraph::prelude::*;` for downstream apps.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// Clippy lints intentionally allowed for this crate:
//
// * `module_inception` — module names mirror the TS source layout
//   (`additional_components::background::background.rs`, …) and the
//   nested file naming makes the source-to-source mapping unambiguous.
// * `large_enum_variant` — many of these enums mirror TS discriminated
//   unions with a heavy "full struct" variant and a lightweight "id"
//   variant. Boxing the heavy variant would change every constructor
//   site downstream; deferred until Phase 2 once the actual API
//   surface settles.
// * `type_complexity` — props bundles necessarily carry deeply-nested
//   generic types that are clearer inline than behind a `type` alias.
// * `doc_overindented_list_items` — comes from copying doc comments
//   verbatim from the TS source.
#![allow(clippy::module_inception)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::type_complexity)]
#![allow(clippy::doc_overindented_list_items)]

pub mod additional_components;
pub mod components;
pub mod container;
pub mod context;
pub mod contexts;
pub mod dom;
pub mod hooks;
pub mod prelude;
pub mod store;
pub mod styles;
pub mod types;
pub mod utils;

// ---------------------------------------------------------------------------
// Public re-exports — mirror `xyflow-react/src/index.ts` 1:1.
// ---------------------------------------------------------------------------

// TODO(rgraph/phase7): re-export `RGraph` once `container::rgraph` is built.
//   TS: `export { default as ReactFlow } from './container/ReactFlow';`
// pub use container::rgraph::RGraph;

// Phase 2: provider + store-updater components.
pub use components::rgraph_provider::{RGraphProvider, RGraphProviderProps};
pub use components::store_updater::{StoreUpdater, StoreUpdaterProps};

// Phase 6: BatchProvider (queue + per-frame flush).
pub use components::batch_provider::{BatchContext, BatchProvider, BatchProviderProps};

// Phase 4 components.
pub use components::a11y_descriptions::{
    A11yDescriptions, A11yDescriptionsProps, ARIA_EDGE_DESC_KEY, ARIA_LIVE_MESSAGE, ARIA_NODE_DESC_KEY,
};
pub use components::attribution::{Attribution, AttributionProps};
pub use components::panel::{Panel, PanelProps};
pub use components::user_selection::{UserSelection, UserSelectionProps};

// Phase 4 containers.
pub use container::pane::{Pane, PaneProps};
pub use container::viewport::{Viewport, ViewportProps};
pub use container::zoom_pane::{ZoomPane, ZoomPaneProps};

// Phase 4 DOM bridges.
pub use dom::{pointer as dom_pointer, wheel as dom_wheel, PaneBounds};

// Phase 5 components.
pub use components::node_wrapper::{
    NodeWrapper,
    NodeWrapperProps as NodeWrapperComponentProps,
};
pub use components::node_wrapper::utils::{arrow_key_diff, BuiltInNodeType, InlineDimensions};
pub use components::nodes::default_node::DefaultNode;
pub use components::nodes::group_node::GroupNode;
pub use components::nodes::input_node::InputNode;
pub use components::nodes::output_node::OutputNode;
pub use components::nodes::utils::{handle_node_click, HandleNodeClickArgs};
pub use components::nodes_selection::{NodesSelection, NodesSelectionProps};
pub use components::selection_listener::{SelectionListener, SelectionListenerProps};

// Phase 5 containers.
pub use container::node_renderer::{NodeRenderer, NodeRendererProps};

// Phase 6 components.
pub use components::connection_line::{
    ConnectionLine, ConnectionLineProps, ConnectionLineWrapper, ConnectionLineWrapperProps,
};
pub use components::edge_label_renderer::{EdgeLabelRenderer, EdgeLabelRendererProps};
pub use components::edge_wrapper::utils::BuiltInEdgeType;
pub use components::edge_wrapper::{EdgeWrapper, EdgeWrapperComponentProps};
pub use components::edge_wrapper::update_anchors::{EdgeUpdateAnchors, EdgeUpdateAnchorsProps};
pub use components::edges::{
    BaseEdge, BaseEdgeComponentProps, BezierEdge, BezierEdgeComponentProps, EdgeAnchor,
    EdgeAnchorProps, EdgeText, EdgeTextProps as EdgeTextComponentProps, SimpleBezierEdge,
    SimpleBezierEdgeComponentProps, SmoothStepEdge, SmoothStepEdgeComponentProps, StepEdge,
    StepEdgeComponentProps, StraightEdge, StraightEdgeComponentProps,
    get_simple_bezier_path, GetSimpleBezierPathParams,
};
pub use components::handle::{Handle, HandleProps as HandleComponentProps};
pub use components::viewport_portal::{ViewportPortal, ViewportPortalProps};

// Phase 6 containers.
pub use container::edge_renderer::marker_definitions::{MarkerDefinitions, MarkerDefinitionsProps};
pub use container::edge_renderer::marker_symbols::{MarkerSymbol, MarkerSymbolProps};
pub use container::edge_renderer::{EdgeRenderer, EdgeRendererProps};

// Phase 3 hooks — public surface mirroring `xyflow-react/src/index.ts`.
pub use hooks::use_color_mode_class::use_color_mode_class;
pub use hooks::use_connection::use_connection;
pub use hooks::use_drag::{use_drag, UseDragParams};
pub use hooks::use_edges::use_edges;
pub use hooks::use_global_key_handler::{
    pressed_signals as global_key_pressed_signals, use_global_key_handler, GlobalKeyHandler,
    GlobalKeyHandlerEffects,
};
pub use hooks::use_handle_connections::{use_handle_connections, UseHandleConnectionsParams};
pub use hooks::use_internal_node::use_internal_node;
pub use hooks::use_key_press::{use_key_press, KeyPressApi, KeyEvent, KeyPressMatcher, UseKeyPressOptions};
pub use hooks::use_move_selected_nodes::{use_move_selected_nodes, MoveSelectedNodes, MoveSelectedNodesParams};
pub use hooks::use_node_connections::{use_node_connections, UseNodeConnectionsParams};
pub use hooks::use_nodes::use_nodes;
pub use hooks::use_nodes_data::{use_node_data, use_nodes_data, NodeDataView};
pub use hooks::use_nodes_edges_state::{use_edges_state, use_nodes_state, UseEdgesState, UseNodesState};
pub use hooks::use_nodes_initialized::{use_nodes_initialized, UseNodesInitializedOptions};
pub use hooks::use_on_edges_change_middleware::experimental_use_on_edges_change_middleware;
pub use hooks::use_on_init_handler::use_on_init_handler;
pub use hooks::use_on_nodes_change_middleware::experimental_use_on_nodes_change_middleware;
pub use hooks::use_on_selection_change::{use_on_selection_change, UseOnSelectionChangeOptions};
pub use hooks::use_on_viewport_change::{use_on_viewport_change, UseOnViewportChangeOptions};
pub use hooks::use_resize_handler::use_resize_handler;
pub use hooks::use_rgraph::{use_rgraph, RGraphHandle};
pub use hooks::use_store::{use_store, use_store_api};
pub use hooks::use_update_node_internals::{use_update_node_internals, UpdateNodeInternals};
pub use hooks::use_viewport::use_viewport;
pub use hooks::use_viewport_helper::{
    use_viewport_helper, PartialViewport, ScreenToFlowOptions, ViewportHelper,
};
pub use hooks::use_viewport_sync::use_viewport_sync;
pub use hooks::use_visible_edge_ids::use_visible_edge_ids;
pub use hooks::use_visible_node_ids::use_visible_node_ids;

// Phase 1 utils — re-export the public functions and helpers.
pub use utils::changes::{apply_edge_changes, apply_node_changes};
pub use utils::general::{is_edge, is_node, Element, PtrEq};

// Phase 1 types — mirror the TS `export * from './types';`.
pub use types::*;

// Phase 2 store + context.
pub use context::{provide_rgraph_store, try_use_rgraph_store, use_rgraph_store};
pub use contexts::node_id::{provide_node_id, use_node_id, NodeIdCtx};
pub use store::{initial_state, InitialStateParams, RGraphStore};

// System types/utils are re-exported from `rgraph_core`.
// TS reference: lines 44–144 of `xyflow-react/src/index.ts`.
pub use rgraph_core as core;
