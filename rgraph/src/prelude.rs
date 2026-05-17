//! `use rgraph::prelude::*;` brings the most commonly used items into scope.
//!
//! Status: Phase 7 — top-level `<RGraph>` added.
//! Items are added here as each later phase lands.

// Phase 7: top-level `<RGraph>` component.
pub use crate::container::rgraph::{RGraph, RGraphProps as RGraphComponentProps};
pub use crate::container::flow_renderer::{FlowRenderer, FlowRendererProps};
pub use crate::container::graph_view::{GraphView, GraphViewProps};
pub use crate::container::rgraph::wrapper::{Wrapper, WrapperProps};

// Phase 1: pure data types and change utilities.
pub use crate::types::edges::{
    BaseEdgeProps, BezierEdgeProps, BuiltInEdge, ConnectionLineComponent, ConnectionLineComponentProps,
    ConnectionStatus, DefaultEdgeOptions, Edge, EdgeComponentProps, EdgeComponentWithPathOptions,
    EdgeLabelOptions, EdgeMouseHandler, EdgeMouseHandlerArgs, EdgePresentation, EdgeProps,
    EdgeReconnectable, EdgeTextProps, EdgeWrapperProps, OnReconnect, OnReconnectArgs, OnReconnectEnd,
    OnReconnectEndArgs, OnReconnectStart, OnReconnectStartArgs, SimpleBezierEdgeProps, SmoothStepEdgeProps,
    StepEdgeProps, StraightEdgeProps,
};
pub use crate::types::general::{
    ConnectionOrEdge, EdgeRenderer, EdgeTypes, FitView, FitViewOptions, FitViewParams, IsValidConnection,
    NodeRenderer, NodeTypes, OnBeforeDelete, OnDelete, OnDeleteArgs, OnEdgesChange, OnEdgesDelete, OnInit,
    OnNodesChange, OnNodesDelete, OnSelectionChangeFunc, OnSelectionChangeParams,
    UnselectNodesAndEdgesParams, ViewportHelperFunctions,
};
pub use crate::types::nodes::{
    BuiltInNode, BuiltInNodeData, InternalNode, MouseData, Node, NodeMouseHandler, NodeMouseHandlerArgs,
    NodePresentation, NodeProps, NodeWrapperProps, OnNodeDrag, OnNodeDragArgs, SelectionDragHandler,
    SelectionDragHandlerArgs,
};
pub use crate::types::instance::{
    DeleteElementsOptions, DeletedElements, EdgeRef, EdgeUpdater, GeneralHelpers, NodeOrIdOrInternal,
    NodeOrRect, NodePartial, NodeRef, NodeUpdater, RGraphInstance, RGraphJsonObject, SetEdgesArg, SetNodesArg,
    UpdateOptions,
};
pub use crate::types::component_props::{
    OnConnect, OnConnectEnd, OnConnectEndCallbackArgs, OnConnectStart, OnConnectStartCallbackArgs, OnError,
    OnErrorArgs, OnMove, OnMoveCallbackArgs, OnMoveEnd, OnMoveStart, OnViewportChange, RGraphProps,
};
pub use crate::utils::changes::{
    apply_edge_changes, apply_node_changes, create_edge_selection_change, create_node_selection_change,
    create_selection_change, edge_to_remove_change, get_elements_diff_changes_edges,
    get_elements_diff_changes_nodes, get_selection_changes_for_edges, get_selection_changes_for_nodes,
    node_to_remove_change,
};
pub use crate::utils::general::{is_edge, is_node, Element, PtrEq};

// Phase 2: store + context + provider.
pub use crate::context::{provide_rgraph_store, try_use_rgraph_store, use_rgraph_store};
pub use crate::contexts::node_id::{provide_node_id, use_node_id, NodeIdCtx};
pub use crate::store::{initial_state, InitialStateParams, RGraphStore};
pub use crate::components::rgraph_provider::{RGraphProvider, RGraphProviderProps};
pub use crate::components::store_updater::{StoreUpdater, StoreUpdaterProps};

// Phase 4: viewport rendering.
pub use crate::components::a11y_descriptions::{A11yDescriptions, A11yDescriptionsProps};
pub use crate::components::attribution::{Attribution, AttributionProps};
pub use crate::components::panel::{Panel, PanelProps};
pub use crate::components::user_selection::{UserSelection, UserSelectionProps};
pub use crate::container::pane::{Pane, PaneProps};
pub use crate::container::viewport::{Viewport, ViewportProps};
pub use crate::container::zoom_pane::{ZoomPane, ZoomPaneProps};
pub use crate::dom::PaneBounds;

// Phase 5: nodes.
pub use crate::components::node_wrapper::{
    NodeWrapper,
    NodeWrapperProps as NodeWrapperComponentProps,
};
pub use crate::components::nodes::default_node::DefaultNode;
pub use crate::components::nodes::group_node::GroupNode;
pub use crate::components::nodes::input_node::InputNode;
pub use crate::components::nodes::output_node::OutputNode;
pub use crate::components::nodes_selection::{NodesSelection, NodesSelectionProps};
pub use crate::components::selection_listener::{SelectionListener, SelectionListenerProps};
pub use crate::container::node_renderer::{NodeRenderer, NodeRendererProps};

// Phase 6: edges + handles + connections.
pub use crate::components::edges::{
    BaseEdge, BaseEdgeComponentProps, BezierEdge, BezierEdgeComponentProps, EdgeAnchor,
    EdgeAnchorProps, EdgeText, SimpleBezierEdge, SimpleBezierEdgeComponentProps, SmoothStepEdge,
    SmoothStepEdgeComponentProps, StepEdge, StepEdgeComponentProps, StraightEdge,
    StraightEdgeComponentProps, get_simple_bezier_path,
};
pub use crate::components::edge_wrapper::{EdgeWrapper, EdgeWrapperComponentProps};
pub use crate::components::handle::{Handle, HandleProps as HandleComponentProps};
pub use crate::components::connection_line::{
    ConnectionLine, ConnectionLineProps, ConnectionLineWrapper, ConnectionLineWrapperProps,
};
pub use crate::components::edge_label_renderer::{EdgeLabelRenderer, EdgeLabelRendererProps};
pub use crate::components::viewport_portal::{ViewportPortal, ViewportPortalProps};
pub use crate::container::edge_renderer::{EdgeRenderer, EdgeRendererProps};
pub use crate::container::edge_renderer::marker_definitions::{MarkerDefinitions, MarkerDefinitionsProps};
pub use crate::container::edge_renderer::marker_symbols::{MarkerSymbol, MarkerSymbolProps};

// Phase 3: hooks.
pub use crate::hooks::use_color_mode_class::use_color_mode_class;
pub use crate::hooks::use_connection::use_connection;
pub use crate::hooks::use_drag::{use_drag, UseDragParams};
pub use crate::hooks::use_edges::use_edges;
pub use crate::hooks::use_global_key_handler::{
    use_global_key_handler, GlobalKeyHandler, GlobalKeyHandlerEffects,
};
pub use crate::hooks::use_handle_connections::{use_handle_connections, UseHandleConnectionsParams};
pub use crate::hooks::use_internal_node::use_internal_node;
pub use crate::hooks::use_key_press::{use_key_press, KeyPressApi, UseKeyPressOptions};
pub use crate::hooks::use_move_selected_nodes::{
    use_move_selected_nodes, MoveSelectedNodes, MoveSelectedNodesParams,
};
pub use crate::hooks::use_node_connections::{use_node_connections, UseNodeConnectionsParams};
pub use crate::hooks::use_nodes::use_nodes;
pub use crate::hooks::use_nodes_data::{use_node_data, use_nodes_data, NodeDataView};
pub use crate::hooks::use_nodes_edges_state::{
    use_edges_state, use_nodes_state, UseEdgesState, UseNodesState,
};
pub use crate::hooks::use_nodes_initialized::{use_nodes_initialized, UseNodesInitializedOptions};
pub use crate::hooks::use_on_edges_change_middleware::experimental_use_on_edges_change_middleware;
pub use crate::hooks::use_on_init_handler::use_on_init_handler;
pub use crate::hooks::use_on_nodes_change_middleware::experimental_use_on_nodes_change_middleware;
pub use crate::hooks::use_on_selection_change::{use_on_selection_change, UseOnSelectionChangeOptions};
pub use crate::hooks::use_on_viewport_change::{use_on_viewport_change, UseOnViewportChangeOptions};
pub use crate::hooks::use_resize_handler::use_resize_handler;
pub use crate::hooks::use_rgraph::{use_rgraph, RGraphHandle};
pub use crate::hooks::use_store::{use_store, use_store_api};
pub use crate::hooks::use_update_node_internals::{use_update_node_internals, UpdateNodeInternals};
pub use crate::hooks::use_viewport::use_viewport;
pub use crate::hooks::use_viewport_helper::{use_viewport_helper, ViewportHelper};
pub use crate::hooks::use_viewport_sync::use_viewport_sync;
pub use crate::hooks::use_visible_edge_ids::use_visible_edge_ids;
pub use crate::hooks::use_visible_node_ids::use_visible_node_ids;

// Phase 8: additional components (Background, Controls, MiniMap,
// NodeToolbar, EdgeToolbar, NodeResizer).
pub use crate::additional_components::background::{
    Background, BackgroundGap, BackgroundOffset, BackgroundProps, BackgroundVariant,
};
pub use crate::additional_components::controls::{
    ControlButton, ControlButtonProps, Controls, ControlsOrientation, ControlsProps, FitViewIcon,
    LockIcon, MinusIcon, PlusIcon, UnlockIcon,
};
pub use crate::additional_components::edge_toolbar::{EdgeToolbar, EdgeToolbarProps};
pub use crate::additional_components::minimap::{
    MiniMap, MiniMapNode, MiniMapNodeAttr, MiniMapNodes, MiniMapProps,
};
pub use crate::additional_components::node_resizer::{
    NodeResizeControl, NodeResizeControlProps, NodeResizer, NodeResizerProps, ResizeControlLine,
};
pub use crate::additional_components::node_toolbar::{
    NodeToolbar, NodeToolbarPortal, NodeToolbarProps, NodeToolbarTarget,
};

// Re-export the core's prelude-like items so consumers only need
// `use rgraph::prelude::*` (instead of pulling from `rgraph_core` too).
pub use rgraph_core as core;
