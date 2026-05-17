//! Port of `xyflow-react/src/types/edges.ts`.
//!
//! Status: Phase 1 — implemented.
//!
//! The TS `Edge` extends `EdgeBase` with React-specific presentation
//! and behavioural fields (`style`, `className`, `reconnectable`,
//! `focusable`, `ariaRole`, `domAttributes`, plus all the
//! `EdgeLabelOptions` fields). Like with [`super::nodes`], we keep
//! [`rgraph_core::types::edges::Edge`] as the canonical `Edge<D>` and
//! collect the React-only presentation fields in a dedicated struct.
//!
//! Path-options enums and edge-specific marker types are re-exported
//! from `rgraph-core`.

#![allow(clippy::module_name_repetitions)]

use dioxus::prelude::{Callback, Element, Event};

use rgraph_core::types::connection::{ConnectionState, FinalConnectionState};
use rgraph_core::types::edges::{
    BezierPathOptions, ConnectionLineType, Edge as EdgeCore, EdgeMarkerType, EdgePosition,
    SmoothStepPathOptions, StepPathOptions,
};
use rgraph_core::types::geometry::{Position, XYPosition};
use rgraph_core::types::handles::{Handle, HandleType};

use crate::types::nodes::{InternalNode, MouseData, Node};

// ---------------------------------------------------------------------------
// Re-export of the canonical edge type.
// ---------------------------------------------------------------------------

/// An `Edge` is the complete description with everything `rgraph` needs
/// in order to render it.
///
/// Re-export of [`rgraph_core::types::edges::Edge`]; React-only
/// presentation overrides live on [`EdgePresentation`].
pub type Edge<D = ()> = EdgeCore<D>;

// ---------------------------------------------------------------------------
// React-only presentation fields on `Edge`.
// ---------------------------------------------------------------------------

/// Additional presentation hints applied by `EdgeWrapper` when it
/// renders an [`Edge`] in the Dioxus DOM. Equivalent to the React-only
/// fields on the TS `Edge` type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EdgePresentation {
    /// Raw `style="..."` snippet for the edge `<path>` element.
    pub style: Option<String>,
    /// Extra class names appended after the framework classes.
    pub class_name: Option<String>,
    /// Whether the edge can be updated by dragging the source or target.
    ///
    /// Overrides `edges_reconnectable` from `<RGraph>` for this edge.
    /// `Some(EdgeReconnectable::Either)` ≅ TS `true`; the `Source` /
    /// `Target` variants pick a single endpoint.
    pub reconnectable: Option<EdgeReconnectable>,
    /// Whether the edge is keyboard-focusable.
    pub focusable: Option<bool>,
    /// The ARIA `role` attribute. Defaults to `"group"` when `None`.
    pub aria_role: Option<String>,
    /// Free-form HTML/SVG attributes appended verbatim to the edge
    /// `<g>` element.
    pub dom_attributes: std::collections::HashMap<String, String>,
}

/// Granular control for `reconnectable` (TS `boolean | HandleType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeReconnectable {
    /// Either endpoint can be moved.
    Either,
    /// Only the source endpoint can be moved.
    Source,
    /// Only the target endpoint can be moved.
    Target,
}

impl From<bool> for EdgeReconnectable {
    /// `true`  → [`EdgeReconnectable::Either`].
    /// `false` → returns [`EdgeReconnectable::Either`] still, because
    /// this conversion is one-way: when the consumer wraps the result
    /// in `Option<EdgeReconnectable>` it should use `None` to express
    /// "disabled". This matches the TS `boolean | HandleType` semantics
    /// where the `false` branch is encoded as the absence of the
    /// `reconnectable` field on the edge.
    fn from(_b: bool) -> Self {
        EdgeReconnectable::Either
    }
}

impl From<HandleType> for EdgeReconnectable {
    fn from(t: HandleType) -> Self {
        match t {
            HandleType::Source => EdgeReconnectable::Source,
            HandleType::Target => EdgeReconnectable::Target,
        }
    }
}

// ---------------------------------------------------------------------------
// EdgeLabelOptions — shared between `Edge`, `EdgeProps` and `BaseEdgeProps`.
// ---------------------------------------------------------------------------

/// Options for rendering an edge's label, used by every built-in edge
/// component and by user-defined edges via [`BaseEdge`].
///
/// Mirrors the TS `EdgeLabelOptions`. In Dioxus, the `label` field is
/// an [`Element`] (a rendered fragment) rather than a TS `ReactNode`.
#[derive(Clone, Default, PartialEq)]
pub struct EdgeLabelOptions {
    /// The label or custom element to render along the edge. This is
    /// commonly a text label or some custom controls.
    pub label: Option<Element>,
    /// Custom `style="..."` snippet to apply to the label `<text>`.
    pub label_style: Option<String>,
    /// Whether to draw a coloured background rectangle behind the label.
    pub label_show_bg: Option<bool>,
    /// Custom `style="..."` snippet to apply to the label background.
    pub label_bg_style: Option<String>,
    /// Padding around the label background `(x, y)` in pixels.
    pub label_bg_padding: Option<(f64, f64)>,
    /// Border radius of the label background rect.
    pub label_bg_border_radius: Option<f64>,
}

impl std::fmt::Debug for EdgeLabelOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeLabelOptions")
            .field("label", &self.label.as_ref().map(|_| "<Element>"))
            .field("label_style", &self.label_style)
            .field("label_show_bg", &self.label_show_bg)
            .field("label_bg_style", &self.label_bg_style)
            .field("label_bg_padding", &self.label_bg_padding)
            .field("label_bg_border_radius", &self.label_bg_border_radius)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Callback aliases.
// ---------------------------------------------------------------------------

/// Click / context-menu / mouse-{enter,move,leave} handler for an edge.
///
/// Mirrors the TS `EdgeMouseHandler = (event, edge) => void`.
pub type EdgeMouseHandler<D = ()> = Callback<EdgeMouseHandlerArgs<D>>;

#[derive(Debug, Clone)]
pub struct EdgeMouseHandlerArgs<D: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub edge: Edge<D>,
}

/// `OnReconnect`. Fires after the user drops a reconnect handle on a
/// new target. Mirrors the TS `OnReconnect = (oldEdge, newConnection) => void`.
pub type OnReconnect<D = ()> = Callback<OnReconnectArgs<D>>;

#[derive(Debug, Clone)]
pub struct OnReconnectArgs<D: Clone = ()> {
    pub old_edge: Edge<D>,
    pub new_connection: rgraph_core::types::connection::Connection,
}

/// `OnReconnectStart`. Fires when the user begins dragging a
/// reconnect anchor.
pub type OnReconnectStart<D = ()> = Callback<OnReconnectStartArgs<D>>;

#[derive(Debug, Clone)]
pub struct OnReconnectStartArgs<D: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub edge: Edge<D>,
    pub handle_type: HandleType,
}

/// `OnReconnectEnd`. Fires when the user finishes (or cancels) a
/// reconnect drag.
pub type OnReconnectEnd<D = (), N = ()> = Callback<OnReconnectEndArgs<D, N>>;

#[derive(Debug, Clone)]
pub struct OnReconnectEndArgs<D: Clone = (), N: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub edge: Edge<D>,
    pub handle_type: HandleType,
    pub connection_state: FinalConnectionState<InternalNode<N>>,
}

// ---------------------------------------------------------------------------
// EdgeWrapperProps.
// ---------------------------------------------------------------------------

/// Props passed by `EdgeRenderer` to the per-edge `<EdgeWrapper>`.
///
/// Mirrors the TS `EdgeWrapperProps`. `edgeTypes` and `onError` are
/// supplied through context in the Rust port and therefore not on
/// this struct. They will be reintroduced if the wrapper API requires
/// them in phase 6.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeWrapperProps<D: Clone = (), N: Clone = ()> {
    pub id: String,
    pub edges_focusable: bool,
    pub edges_reconnectable: bool,
    pub elements_selectable: bool,
    pub no_pan_class_name: String,
    pub on_click: Option<EdgeMouseHandler<D>>,
    pub on_double_click: Option<EdgeMouseHandler<D>>,
    pub on_reconnect: Option<OnReconnect<D>>,
    pub on_context_menu: Option<EdgeMouseHandler<D>>,
    pub on_mouse_enter: Option<EdgeMouseHandler<D>>,
    pub on_mouse_move: Option<EdgeMouseHandler<D>>,
    pub on_mouse_leave: Option<EdgeMouseHandler<D>>,
    pub reconnect_radius: Option<f64>,
    pub on_reconnect_start: Option<OnReconnectStart<D>>,
    pub on_reconnect_end: Option<OnReconnectEnd<D, N>>,
    pub rf_id: Option<String>,
    pub disable_keyboard_a11y: Option<bool>,
}

// ---------------------------------------------------------------------------
// DefaultEdgeOptions.
// ---------------------------------------------------------------------------

/// Many properties on an [`Edge`] are optional. When a new edge is
/// created, the properties that are not provided are filled in with
/// the defaults from `<RGraph default_edge_options="…">`.
///
/// Mirrors the TS `DefaultEdgeOptions = DefaultEdgeOptionsBase<Edge>`.
/// In Rust we model it as a struct of optional fields that
/// `set_edges`/`add_edges` can merge into a new [`Edge`] before
/// inserting it. Only fields that make sense as cross-edge defaults
/// are exposed (matches the TS shape exactly).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DefaultEdgeOptions {
    pub type_: Option<String>,
    pub animated: Option<bool>,
    pub hidden: Option<bool>,
    pub deletable: Option<bool>,
    pub focusable: Option<bool>,
    pub selectable: Option<bool>,
    pub marker_start: Option<EdgeMarkerType>,
    pub marker_end: Option<EdgeMarkerType>,
    pub z_index: Option<i32>,
    pub style: Option<String>,
    pub class_name: Option<String>,
    pub interaction_width: Option<f64>,
    pub reconnectable: Option<EdgeReconnectable>,
}

// ---------------------------------------------------------------------------
// EdgeProps — passed to user-defined edge components.
// ---------------------------------------------------------------------------

/// Props passed to a user-defined custom edge component.
///
/// Mirrors the TS `EdgeProps<EdgeType>` (a `Pick` from `Edge` plus
/// `EdgePosition` and [`EdgeLabelOptions`]).
///
/// `PathOptions` is generic: the TS uses `any` for `pathOptions`, but
/// in Rust we keep it typed so custom edges can declare exactly the
/// variant they expect (e.g. `EdgeProps<MyData, BezierPathOptions>`).
/// The default `()` mirrors "no path options".
#[derive(Clone, PartialEq)]
pub struct EdgeProps<D: Clone = (), PathOptions: Clone + PartialEq = ()> {
    pub id: String,
    pub type_: Option<String>,
    pub animated: Option<bool>,
    pub data: D,
    pub style: Option<String>,
    pub selected: Option<bool>,
    pub source: String,
    pub target: String,
    pub selectable: Option<bool>,
    pub deletable: Option<bool>,
    pub source_handle_id: Option<String>,
    pub target_handle_id: Option<String>,
    pub marker_start: Option<String>,
    pub marker_end: Option<String>,
    pub path_options: Option<PathOptions>,
    pub interaction_width: Option<f64>,
    // EdgePosition (flattened):
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub source_position: Position,
    pub target_position: Position,
    pub label_options: EdgeLabelOptions,
}

impl<D: Clone + std::fmt::Debug, P: Clone + PartialEq + std::fmt::Debug> std::fmt::Debug for EdgeProps<D, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeProps")
            .field("id", &self.id)
            .field("type_", &self.type_)
            .field("animated", &self.animated)
            .field("data", &self.data)
            .field("selected", &self.selected)
            .field("source", &self.source)
            .field("target", &self.target)
            .field("source_x", &self.source_x)
            .field("source_y", &self.source_y)
            .field("target_x", &self.target_x)
            .field("target_y", &self.target_y)
            .field("source_position", &self.source_position)
            .field("target_position", &self.target_position)
            .field("path_options", &self.path_options)
            .finish_non_exhaustive()
    }
}

impl<D: Clone, P: Clone + PartialEq> EdgeProps<D, P> {
    /// Convenience accessor — the `EdgePosition` portion of these props.
    #[inline]
    #[must_use]
    pub fn edge_position(&self) -> EdgePosition {
        EdgePosition {
            source_x: self.source_x,
            source_y: self.source_y,
            target_x: self.target_x,
            target_y: self.target_y,
            source_position: self.source_position,
            target_position: self.target_position,
        }
    }
}

// ---------------------------------------------------------------------------
// BaseEdgeProps — props for the shared `<BaseEdge>` building block.
// ---------------------------------------------------------------------------

/// Props for the [`BaseEdge`] component (the SVG `<path>` + optional
/// label/markers).
///
/// Mirrors the TS `BaseEdgeProps`. The `Omit<SVGAttributes<…>, ...>`
/// pass-through is modelled as a `HashMap<String, String>` on
/// [`EdgePresentation::dom_attributes`] supplied by the calling edge
/// component, plus an explicit `style` field here.
#[derive(Clone, PartialEq)]
pub struct BaseEdgeProps {
    /// The SVG path string (e.g. `"M 0 0 L 100 100"`). Required.
    pub path: String,
    /// Inline `style="..."` snippet for the visible path.
    pub style: Option<String>,
    /// Class name for the visible path.
    pub class_name: Option<String>,
    /// Width of the invisible hit area around the path. TS default is 20.
    pub interaction_width: Option<f64>,
    /// X-position of the edge label.
    pub label_x: Option<f64>,
    /// Y-position of the edge label.
    pub label_y: Option<f64>,
    /// Marker URL string used at the start of the path (`"url(#id)"`).
    pub marker_start: Option<String>,
    /// Marker URL string used at the end of the path (`"url(#id)"`).
    pub marker_end: Option<String>,
    /// Shared label-rendering options.
    pub label_options: EdgeLabelOptions,
}

impl std::fmt::Debug for BaseEdgeProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseEdgeProps")
            .field("path", &self.path)
            .field("style", &self.style)
            .field("class_name", &self.class_name)
            .field("interaction_width", &self.interaction_width)
            .field("label_x", &self.label_x)
            .field("label_y", &self.label_y)
            .field("marker_start", &self.marker_start)
            .field("marker_end", &self.marker_end)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Built-in edge components' prop bundles.
// ---------------------------------------------------------------------------

/// Common shape of every built-in edge component's props.
///
/// Mirrors the TS `EdgeComponentProps = EdgePosition & EdgeLabelOptions
/// & { id, markerStart, markerEnd, interactionWidth, style,
/// sourceHandleId, targetHandleId }`.
#[derive(Clone, PartialEq)]
pub struct EdgeComponentProps {
    pub id: Option<String>,
    pub marker_start: Option<String>,
    pub marker_end: Option<String>,
    pub interaction_width: Option<f64>,
    pub style: Option<String>,
    pub source_handle_id: Option<String>,
    pub target_handle_id: Option<String>,
    // EdgePosition (flattened):
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub source_position: Position,
    pub target_position: Position,
    pub label_options: EdgeLabelOptions,
}

impl std::fmt::Debug for EdgeComponentProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeComponentProps")
            .field("id", &self.id)
            .field("source_position", &self.source_position)
            .field("target_position", &self.target_position)
            .finish_non_exhaustive()
    }
}

/// `EdgeComponentProps` plus a `path_options` slot, parameterised over
/// the concrete options type.
///
/// Mirrors the TS `EdgeComponentWithPathOptions<PathOptions>`.
#[derive(Clone, PartialEq)]
pub struct EdgeComponentWithPathOptions<PathOptions: Clone + PartialEq> {
    pub common: EdgeComponentProps,
    pub path_options: Option<PathOptions>,
}

impl<P: Clone + PartialEq + std::fmt::Debug> std::fmt::Debug for EdgeComponentWithPathOptions<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeComponentWithPathOptions")
            .field("common", &self.common)
            .field("path_options", &self.path_options)
            .finish()
    }
}

/// Props for the [`BezierEdge`] built-in.
pub type BezierEdgeProps = EdgeComponentWithPathOptions<BezierPathOptions>;

/// Props for the [`SmoothStepEdge`] built-in.
pub type SmoothStepEdgeProps = EdgeComponentWithPathOptions<SmoothStepPathOptions>;

/// Props for the [`StepEdge`] built-in.
pub type StepEdgeProps = EdgeComponentWithPathOptions<StepPathOptions>;

/// Props for the [`StraightEdge`] built-in. TS omits the source/target
/// `Position`s — we keep [`EdgeComponentProps`] but downstream
/// components ignore those two fields for straight edges.
pub type StraightEdgeProps = EdgeComponentProps;

/// Props for the [`SimpleBezierEdge`] built-in.
pub type SimpleBezierEdgeProps = EdgeComponentProps;

/// Props for an edge-rendered `EdgeText` SVG label.
///
/// Mirrors the TS `EdgeTextProps`. The `x` and `y` are required; the
/// remaining presentational fields are reused from
/// [`EdgeLabelOptions`].
#[derive(Clone, PartialEq, Default)]
pub struct EdgeTextProps {
    pub x: f64,
    pub y: f64,
    pub label_options: EdgeLabelOptions,
}

impl std::fmt::Debug for EdgeTextProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EdgeTextProps")
            .field("x", &self.x)
            .field("y", &self.y)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// BuiltInEdge.
// ---------------------------------------------------------------------------

/// Discriminated union of the four built-in edge variants
/// (`smoothstep`, `default` (bezier), `step`, `straight`).
///
/// Mirrors the TS `BuiltInEdge`.
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltInEdge<D: Clone = ()> {
    SmoothStep(Edge<D>, SmoothStepPathOptions),
    Bezier(Edge<D>, BezierPathOptions),
    Step(Edge<D>, StepPathOptions),
    Straight(Edge<D>),
}

impl<D: Clone> BuiltInEdge<D> {
    /// Borrow as a generic [`Edge<D>`].
    #[must_use]
    pub fn as_edge(&self) -> &Edge<D> {
        match self {
            BuiltInEdge::SmoothStep(e, _)
            | BuiltInEdge::Bezier(e, _)
            | BuiltInEdge::Step(e, _)
            | BuiltInEdge::Straight(e) => e,
        }
    }
}

// ---------------------------------------------------------------------------
// ConnectionLineComponentProps.
// ---------------------------------------------------------------------------

/// Connection-status enum used by [`ConnectionLineComponentProps`].
///
/// Mirrors the TS `'valid' | 'invalid' | null`. Encoded as
/// `Option<ConnectionStatus>` in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Valid,
    Invalid,
}

/// Props for a custom connection-line component supplied via the
/// `connection_line_component` prop on `<RGraph>`.
///
/// Mirrors the TS `ConnectionLineComponentProps<NodeType>`.
#[derive(Clone, PartialEq)]
pub struct ConnectionLineComponentProps<N: Clone = ()> {
    pub connection_line_style: Option<String>,
    pub connection_line_type: ConnectionLineType,
    pub from_node: InternalNode<N>,
    pub from_handle: Handle,
    pub from_x: f64,
    pub from_y: f64,
    pub to_x: f64,
    pub to_y: f64,
    pub from_position: Position,
    pub to_position: Position,
    pub connection_status: Option<ConnectionStatus>,
    pub to_node: Option<InternalNode<N>>,
    pub to_handle: Option<Handle>,
    pub pointer: XYPosition,
}

impl<N: Clone + std::fmt::Debug> std::fmt::Debug for ConnectionLineComponentProps<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionLineComponentProps")
            .field("connection_line_type", &self.connection_line_type)
            .field("from_x", &self.from_x)
            .field("from_y", &self.from_y)
            .field("to_x", &self.to_x)
            .field("to_y", &self.to_y)
            .field("connection_status", &self.connection_status)
            .finish_non_exhaustive()
    }
}

/// Component type for a custom connection-line renderer.
///
/// Mirrors the TS `ConnectionLineComponent<NodeType> =
/// ComponentType<ConnectionLineComponentProps<NodeType>>`. In Dioxus
/// any function `fn(ConnectionLineComponentProps<N>) -> Element` works;
/// we type-erase it with a `Callback`-shaped wrapper so the props bag
/// stays `PartialEq`.
pub type ConnectionLineComponent<N = ()> = Callback<ConnectionLineComponentProps<N>, Element>;

/// Re-export the live connection state for convenience (custom
/// connection-line components occasionally want to inspect it).
pub use rgraph_core::types::connection::ConnectionState as LiveConnectionState;
// suppress unused-import warning when the `core::ConnectionState` is
// pulled in for the type re-export above.
#[allow(unused_imports)]
use ConnectionState as _;

#[cfg(test)]
mod tests {
    use super::*;
    use rgraph_core::Edge as CoreEdge;

    #[test]
    fn edge_alias_is_core_edge() {
        let _e: Edge<()> = CoreEdge::<()>::minimal("e1", "a", "b");
    }

    #[test]
    fn default_edge_options_is_all_none() {
        let d = DefaultEdgeOptions::default();
        assert!(d.type_.is_none());
        assert!(d.animated.is_none());
        assert!(d.marker_start.is_none());
    }

    #[test]
    fn edge_reconnectable_from_handle_type() {
        assert_eq!(EdgeReconnectable::from(HandleType::Source), EdgeReconnectable::Source);
        assert_eq!(EdgeReconnectable::from(HandleType::Target), EdgeReconnectable::Target);
    }

    #[test]
    fn built_in_edge_borrows_inner() {
        let e: Edge<()> = CoreEdge::minimal("e1", "a", "b");
        let b = BuiltInEdge::Bezier(e.clone(), BezierPathOptions::default());
        assert_eq!(b.as_edge().id, "e1");
    }

    #[test]
    fn edge_props_position_round_trip() {
        let p = EdgeProps::<()> {
            id: "e1".into(),
            type_: None,
            animated: None,
            data: (),
            style: None,
            selected: None,
            source: "a".into(),
            target: "b".into(),
            selectable: None,
            deletable: None,
            source_handle_id: None,
            target_handle_id: None,
            marker_start: None,
            marker_end: None,
            path_options: None,
            interaction_width: None,
            source_x: 1.0,
            source_y: 2.0,
            target_x: 3.0,
            target_y: 4.0,
            source_position: Position::Right,
            target_position: Position::Left,
            label_options: EdgeLabelOptions::default(),
        };
        let ep = p.edge_position();
        assert_eq!(ep.source_x, 1.0);
        assert_eq!(ep.target_position, Position::Left);
    }
}
