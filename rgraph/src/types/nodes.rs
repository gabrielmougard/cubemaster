//! Port of `xyflow-react/src/types/nodes.ts`.
//!
//! Status: Phase 1 — implemented.
//!
//! The TS `Node` type extends `NodeBase` with React-specific
//! presentation fields:
//!
//! * `style?: CSSProperties`
//! * `className?: string`
//! * `resizing?: boolean`
//! * `focusable?: boolean`
//! * `ariaRole?: AriaRole`
//! * `domAttributes?: …`
//!
//! In the Dioxus port we keep [`rgraph_core::types::nodes::Node`] —
//! which already carries every behavioural field — as the canonical
//! `Node<D>` and re-export it here verbatim. The React-only
//! *presentation* fields (`style`, `className`, …) are gathered in a
//! separate [`NodePresentation`] struct that custom node renderers can
//! read off the wrapper, mirroring how the TS code spreads them onto
//! the `<div>` wrapper. This keeps `Node<D>` serialisable as plain
//! JSON without ever touching DOM-level types.

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dioxus::prelude::{Callback, Event};
// The `Props` derive macro's generated code references
// `dioxus_core::*` paths directly. We don't depend on `dioxus_core`
// from `Cargo.toml`, so we re-alias it from the `dioxus` re-export.
#[allow(unused_imports)]
use dioxus::dioxus_core;

use rgraph_core::types::geometry::{CoordinateExtent, XYPosition};
use rgraph_core::types::nodes::{InternalNode as InternalNodeCore, Node as NodeCore};

// ---------------------------------------------------------------------------
// Re-exports of the canonical node types from `rgraph-core`.
// ---------------------------------------------------------------------------

/// The `Node` type represents everything `rgraph` needs to know about a
/// given node. Whenever you want to update a certain attribute of a
/// node, you need to create a new node object.
///
/// This is a direct re-export of [`rgraph_core::types::nodes::Node`];
/// the React-specific presentation extensions live on
/// [`NodePresentation`].
pub type Node<D = ()> = NodeCore<D>;

/// The `InternalNode` type is identical to the base [`Node`] but
/// extended with internal bookkeeping (`position_absolute`, `z`,
/// `handle_bounds`, …) used by the store.
///
/// Re-export of [`rgraph_core::types::nodes::InternalNode`].
pub type InternalNode<D = ()> = InternalNodeCore<D>;

// ---------------------------------------------------------------------------
// React-only presentation fields, split out so `Node<D>` stays a pure
// data structure.
// ---------------------------------------------------------------------------

/// Additional presentation hints applied by `NodeWrapper` when it
/// renders a [`Node`] in the Dioxus DOM. Equivalent to the React-only
/// fields on the TS `Node` type that have no behavioural meaning.
///
/// All fields are optional; the wrapper only emits the attributes that
/// have been set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodePresentation {
    /// Raw `style="..."` snippet appended after the framework-controlled
    /// styles (TS `style?: CSSProperties`).
    pub style: Option<String>,
    /// Extra class names appended after the framework classes
    /// (TS `className?: string`).
    pub class_name: Option<String>,
    /// `true` while the node is currently being resized by `NodeResizer`.
    pub resizing: Option<bool>,
    /// Whether the node receives keyboard focus. Defaults to the
    /// `nodesFocusable` flag on `<RGraph>` when `None`.
    pub focusable: Option<bool>,
    /// The ARIA `role` attribute for the node element. Defaults to
    /// `"group"` when `None` (matching the TS default).
    pub aria_role: Option<String>,
    /// Free-form HTML attributes appended verbatim to the node wrapper
    /// `<div>`. Used as the Dioxus equivalent of TS `domAttributes`.
    pub dom_attributes: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Callback aliases.
// ---------------------------------------------------------------------------

/// Mouse/pointer event payload exposed by the Dioxus DOM in handlers.
///
/// We re-export Dioxus' own [`dioxus::events::MouseData`] under a stable
/// crate name so the public types in `rgraph` don't change shape if
/// upstream renames it later.
pub use dioxus::events::MouseData;

/// Click / context-menu / mouse-{enter,move,leave} handler for a node.
///
/// Mirrors the TS `NodeMouseHandler = (event, node) => void`.
pub type NodeMouseHandler<D = ()> =
    Callback<NodeMouseHandlerArgs<D>>;

/// Tuple-like argument bundle for [`NodeMouseHandler`]. We pass a
/// dedicated struct (instead of multiple positional args) so that
/// Dioxus' `Callback<Args>` carries a single arg type, matching its
/// public API.
///
/// `Event<MouseData>` doesn't implement `PartialEq`, so neither does
/// this struct; that is fine because handler arguments are constructed
/// at call sites only — they never appear inside `#[derive(Props)]`
/// bags themselves (which would require `PartialEq`).
#[derive(Debug, Clone)]
pub struct NodeMouseHandlerArgs<D: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub node: Node<D>,
}

/// Selection-drag handler: receives the event plus the full list of
/// currently-selected nodes.
///
/// Mirrors the TS `SelectionDragHandler = (event, nodes) => void`.
pub type SelectionDragHandler<D = ()> =
    Callback<SelectionDragHandlerArgs<D>>;

#[derive(Debug, Clone)]
pub struct SelectionDragHandlerArgs<D: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub nodes: Vec<Node<D>>,
}

/// Node-drag handler. Receives the event, the node being dragged, and
/// the full list of nodes being moved together.
///
/// Mirrors the TS `OnNodeDrag = (event, node, nodes) => void`.
pub type OnNodeDrag<D = ()> = Callback<OnNodeDragArgs<D>>;

#[derive(Debug, Clone)]
pub struct OnNodeDragArgs<D: Clone = ()> {
    pub event: std::rc::Rc<Event<MouseData>>,
    pub node: Node<D>,
    pub nodes: Vec<Node<D>>,
}

// ---------------------------------------------------------------------------
// NodeWrapperProps.
// ---------------------------------------------------------------------------

/// Props passed by `NodeRenderer` to the per-node `<NodeWrapper>`.
///
/// Mirrors the TS `NodeWrapperProps`. Component-level usage (phase 5)
/// will wrap this in `#[derive(dioxus::prelude::Props)]`; for phase 1
/// it is a plain `Clone`/`PartialEq` struct documenting the contract.
///
/// The `resizeObserver` field in TS is replaced by a node-id-keyed
/// `dom::resize_observer` handle managed by the wrapper at runtime; it
/// does not appear on this prop bag.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeWrapperProps<D: Clone = ()> {
    pub id: String,
    pub nodes_connectable: bool,
    pub elements_selectable: bool,
    pub nodes_draggable: bool,
    pub nodes_focusable: bool,
    pub on_click: Option<NodeMouseHandler<D>>,
    pub on_double_click: Option<NodeMouseHandler<D>>,
    pub on_mouse_enter: Option<NodeMouseHandler<D>>,
    pub on_mouse_move: Option<NodeMouseHandler<D>>,
    pub on_mouse_leave: Option<NodeMouseHandler<D>>,
    pub on_context_menu: Option<NodeMouseHandler<D>>,
    pub no_drag_class_name: String,
    pub no_pan_class_name: String,
    pub rf_id: String,
    pub disable_keyboard_a11y: bool,
    pub node_extent: Option<CoordinateExtent>,
    /// Maximum click-vs-drag movement, in pixels.
    pub node_click_distance: Option<f64>,
}

// ---------------------------------------------------------------------------
// BuiltInNode.
// ---------------------------------------------------------------------------

/// Discriminated union of the four built-in node variants
/// (`input`, `output`, `default`, `group`).
///
/// Mirrors the TS `BuiltInNode`:
/// ```ts
/// type BuiltInNode =
///   | Node<{ label: string }, 'input' | 'output' | 'default' | undefined>
///   | Node<Record<string, never>, 'group'>;
/// ```
///
/// The Rust port exposes a `Label`-bearing data type
/// ([`BuiltInNodeData::Labelled`]) for the first three variants and a
/// unit-data type ([`BuiltInNodeData::Empty`]) for `group`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum BuiltInNodeData {
    /// Data payload of `input`, `output`, and `default` nodes.
    Labelled { label: String },
    /// Data payload of `group` nodes (empty).
    #[default]
    Empty,
}

/// Either a labelled-data node (`input` / `output` / `default`) or a
/// `group` node. Each variant carries a fully-formed [`Node`] so it can
/// be inserted into a `Vec<Node<BuiltInNodeData>>` after `From`-conversion.
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltInNode {
    Input(Node<BuiltInNodeData>),
    Output(Node<BuiltInNodeData>),
    Default(Node<BuiltInNodeData>),
    Group(Node<BuiltInNodeData>),
}

impl BuiltInNode {
    /// Borrow as a generic [`Node<BuiltInNodeData>`].
    #[must_use]
    pub fn as_node(&self) -> &Node<BuiltInNodeData> {
        match self {
            BuiltInNode::Input(n) | BuiltInNode::Output(n) | BuiltInNode::Default(n) | BuiltInNode::Group(n) => n,
        }
    }
}

// ---------------------------------------------------------------------------
// NodeProps — props passed to user-defined custom node components.
// ---------------------------------------------------------------------------

/// Props passed to a user-defined custom node component. Mirrors the
/// TS `NodeProps<NodeType>`. Every field is essentially a flattened
/// view of [`Node`] + the live "this is currently selected/dragging…"
/// flags computed by the wrapper.
///
/// The TS version exposes a `Pick<NodeType, …>`-style subset; we expose
/// the same fields by spelling them out, so custom nodes don't need to
/// pull in `rgraph_core` directly.
#[derive(dioxus::prelude::Props, Debug, Clone, PartialEq)]
pub struct NodeProps<D: Clone + PartialEq + 'static = ()> {
    pub id: String,
    pub type_: Option<String>,
    pub data: D,
    pub selected: Option<bool>,
    pub dragging: Option<bool>,
    pub is_connectable: Option<bool>,
    pub x_pos: f64,
    pub y_pos: f64,
    pub z_index: Option<i32>,
    pub source_position: Option<rgraph_core::types::geometry::Position>,
    pub target_position: Option<rgraph_core::types::geometry::Position>,
    pub drag_handle: Option<String>,
    pub parent_id: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub deletable: Option<bool>,
    pub selectable: Option<bool>,
}

impl<D: Clone + PartialEq + 'static> NodeProps<D> {
    /// Convenience accessor — the `(x, y)` position in flow space.
    #[inline]
    #[must_use]
    pub fn position(&self) -> XYPosition {
        XYPosition::new(self.x_pos, self.y_pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_alias_is_core_node() {
        // If this compiles the alias is correct.
        let _n: Node<()> = Node::<()>::minimal("n1", 0.0, 0.0);
    }

    #[test]
    fn node_presentation_default_has_no_attributes() {
        let p = NodePresentation::default();
        assert!(p.style.is_none());
        assert!(p.class_name.is_none());
        assert!(p.dom_attributes.is_empty());
    }

    #[test]
    fn builtin_node_default_data_is_empty() {
        assert!(matches!(BuiltInNodeData::default(), BuiltInNodeData::Empty));
    }
}
