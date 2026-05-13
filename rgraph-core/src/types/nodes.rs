//! Port of `xyflow-core/src/types/nodes.ts`.
//!
//! Status: implemented (phase 1).

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::geometry::{CoordinateExtent, Position, XYPosition};
use crate::types::handles::{Handle, HandleType};

/// Origin of a node relative to its position.
///
/// `[0.0, 0.0]` places the node at the top-left of its position;
/// `[0.5, 0.5]` centers it; `[1.0, 1.0]` aligns it to the bottom-right.
pub type NodeOrigin = (f64, f64);

/// Optional measurements set by the renderer once a node mounts.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MeasuredDimensions {
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// Boundary a node can be moved within.
///
/// Mirrors the TS `'parent' | CoordinateExtent | null`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum NodeExtent {
    /// No extent — node can move freely.
    #[default]
    Unbounded,
    /// Constrained to the parent node's rectangle.
    Parent,
    /// Explicit coordinate extent.
    Custom(CoordinateExtent),
}

#[cfg(feature = "serde")]
mod node_extent_serde {
    //! Custom (de)serialization to mirror the JS shape:
    //! `'parent'` (string), `null` / `undefined` (Unbounded), or `[[x,y],[x,y]]`.
    use super::*;
    use serde::de::Error;
    use serde::ser::Serializer;
    use serde::{Deserialize, Deserializer, Serialize};

    impl Serialize for NodeExtent {
        fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            match self {
                NodeExtent::Unbounded => ser.serialize_none(),
                NodeExtent::Parent => "parent".serialize(ser),
                NodeExtent::Custom(c) => c.serialize(ser),
            }
        }
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Wire {
        Parent(String),
        Custom(CoordinateExtent),
    }

    impl<'de> Deserialize<'de> for NodeExtent {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            let wire = Option::<Wire>::deserialize(de)?;
            match wire {
                None => Ok(NodeExtent::Unbounded),
                Some(Wire::Parent(s)) if s == "parent" => Ok(NodeExtent::Parent),
                Some(Wire::Parent(s)) => Err(D::Error::custom(format!(
                    "expected 'parent' or coordinate extent, got '{s}'"
                ))),
                Some(Wire::Custom(c)) => Ok(NodeExtent::Custom(c)),
            }
        }
    }
}

/// Subset of [`Handle`] attached to a [`Node`] declaration before the
/// node has mounted.
///
/// Mirrors the TS `NodeHandle = Omit<Optional<Handle, 'width'|'height'>, 'nodeId'>`:
/// width/height are optional (the renderer may measure them) and the
/// `node_id` is implicit.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeHandle {
    #[cfg_attr(feature = "serde", serde(default))]
    pub id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub position: Position,
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub type_: HandleType,
    #[cfg_attr(feature = "serde", serde(default))]
    pub width: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub height: Option<f64>,
}

/// Framework-independent node data structure.
///
/// Generic over user-supplied data type `D`. The TS `type` field is
/// renamed to `type_` to dodge the Rust keyword and remains optional —
/// the field accepts an arbitrary string identifying a `nodeType`
/// registered with the renderer.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Node<D: Clone = ()> {
    pub id: String,
    pub position: XYPosition,
    pub data: D,
    /// Source side for default/source/target node types.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_position: Option<Position>,
    /// Target side for default/source/target node types.
    #[cfg_attr(feature = "serde", serde(default))]
    pub target_position: Option<Position>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub dragging: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub draggable: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selectable: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub connectable: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub deletable: Option<bool>,
    /// Class selector that turns inner elements into drag handles.
    #[cfg_attr(feature = "serde", serde(default))]
    pub drag_handle: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub width: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub height: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub initial_width: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub initial_height: Option<f64>,
    /// Parent node id, used for sub-flows.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parent_id: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub z_index: Option<i32>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub extent: NodeExtent,
    /// Whether the parent should auto-grow when this node is dragged
    /// past its bounds.
    #[cfg_attr(feature = "serde", serde(default))]
    pub expand_parent: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub aria_label: Option<String>,
    /// Origin relative to position. `[0,0]`=top-left, `[0.5,0.5]`=center,
    /// `[1,1]`=bottom-right.
    #[cfg_attr(feature = "serde", serde(default))]
    pub origin: Option<NodeOrigin>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub handles: Option<Vec<NodeHandle>>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub measured: Option<MeasuredDimensions>,
    /// Type of node defined in `nodeTypes`.
    #[cfg_attr(feature = "serde", serde(rename = "type", default))]
    pub type_: Option<String>,
}

impl<D: Clone + Default> Node<D> {
    /// Convenience for tests: minimal node from id and position.
    #[must_use]
    pub fn minimal(id: impl Into<String>, x: f64, y: f64) -> Self {
        Node {
            id: id.into(),
            position: XYPosition::new(x, y),
            data: D::default(),
            source_position: None,
            target_position: None,
            hidden: None,
            selected: None,
            dragging: None,
            draggable: None,
            selectable: None,
            connectable: None,
            deletable: None,
            drag_handle: None,
            width: None,
            height: None,
            initial_width: None,
            initial_height: None,
            parent_id: None,
            z_index: None,
            extent: NodeExtent::Unbounded,
            expand_parent: None,
            aria_label: None,
            origin: None,
            handles: None,
            measured: None,
            type_: None,
        }
    }
}

/// Source/target lists of measured handles attached to a node.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeHandleBounds {
    pub source: Option<Vec<Handle>>,
    pub target: Option<Vec<Handle>>,
}

/// Measured node bounds, with `Option`-wrapped width/height matching
/// the TS shape.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeBounds {
    pub position: XYPosition,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// Internal-only bookkeeping attached to every adopted node.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NodeInternals {
    /// Position resolved through the parent chain.
    pub position_absolute: XYPosition,
    /// Resolved z-index.
    pub z: f64,
    pub root_parent_index: Option<usize>,
    pub handle_bounds: Option<NodeHandleBounds>,
    pub bounds: Option<NodeBounds>,
}

/// Adopted node — the user-supplied [`Node`] together with renderer
/// bookkeeping in [`NodeInternals`].
///
/// Mirrors the TS `InternalNodeBase = Omit<NodeType, 'measured'> & {
///   measured: Required<…>; internals: { … } }`.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalNode<D: Clone = ()> {
    /// The user-facing node — [TS] `userNode` lives inside `internals`,
    /// but the rest of the TS shape is the user node spread on top. We
    /// flatten back to "user node + internals" here so callers just
    /// borrow `&internal.user`.
    pub user: Node<D>,
    /// Concrete measured dimensions (TS makes `measured` non-optional
    /// on the internal type, with `Option<f64>` fields).
    pub measured: MeasuredDimensions,
    /// Renderer-managed internals.
    pub internals: NodeInternals,
}

impl<D: Clone + Default> InternalNode<D> {
    /// Build an `InternalNode` by adopting `user`, copying its
    /// `position` to `position_absolute` (the rest is filled in by the
    /// store).
    #[must_use]
    pub fn from_user(user: Node<D>) -> Self {
        let measured = user.measured.unwrap_or_default();
        let position_absolute = user.position;
        InternalNode {
            measured,
            internals: NodeInternals {
                position_absolute,
                ..NodeInternals::default()
            },
            user,
        }
    }
}

/// Describes an in-progress drag of a single node.
///
/// Mirrors the TS `NodeDragItem`. `extent`, `parent_id`, `origin`,
/// `expand_parent`, and `dragging` are `Pick`s from `InternalNodeBase`
/// — ported as their owned counterparts so the drag item is
/// self-contained.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeDragItem {
    pub id: String,
    pub position: XYPosition,
    /// Distance from the cursor to the node when the drag began.
    pub distance: XYPosition,
    pub measured: Dimensions,
    pub position_absolute: XYPosition,
    pub extent: NodeExtent,
    pub parent_id: Option<String>,
    pub origin: Option<NodeOrigin>,
    pub expand_parent: Option<bool>,
    pub dragging: Option<bool>,
}

// Re-import here so we can spell `Dimensions` in `NodeDragItem` above
// without dragging a longer path through.
use crate::types::geometry::Dimensions;

/// Update payload sent to the store when a node mounts / its DOM size
/// changes.
///
/// Note: TS holds an `HTMLDivElement` reference. The Rust port relies
/// on the caller having already measured the bounding rect (Dioxus
/// crate) and embedding the measurements into the message via separate
/// channels — therefore this struct only carries the identifier and
/// the `force` bit.
#[derive(Debug, Clone, PartialEq)]
pub struct InternalNodeUpdate {
    pub id: String,
    pub force: Option<bool>,
}

/// Selection-drag callback payload alias.
///
/// Ported as a boxed callback rather than a Rust type alias because the
/// callable nature of the TS `OnSelectionDrag` doesn't translate.
pub type OnSelectionDragHandler<D> =
    Box<dyn FnMut(&crate::types::nodes::PointerEventLike, &[Node<D>]) + Send + Sync>;

/// Placeholder pointer-event view used by callback signatures that need
/// an event in their payload.
///
/// Filled in (or replaced) when `xypanzoom`/`xydrag` define their full
/// `PointerEventLike` in phase 4/5. Consumers can use the public
/// fields here to read screen-space coordinates and modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointerEventLike {
    pub client_x: f64,
    pub client_y: f64,
    pub button: u8,
    pub buttons: u8,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
}

/// Alignment of the node toolbar relative to the node's anchor side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Align {
    #[default]
    Center,
    Start,
    End,
}

/// `id -> InternalNode<D>` lookup mirroring the TS `NodeLookup`.
pub type NodeLookup<D = ()> = HashMap<String, InternalNode<D>>;

/// `parent_id -> children_lookup` mirroring the TS `ParentLookup`.
pub type ParentLookup<D = ()> = HashMap<String, HashMap<String, InternalNode<D>>>;

/// Read-only trait giving uniform access to the geometry shared by both
/// [`Node`] and [`InternalNode`]. Used by helpers like `node_to_rect`
/// in `utils::general` (phase 2) which the TS source overloads via a
/// type guard.
pub trait NodeLike {
    fn id(&self) -> &str;
    fn position(&self) -> XYPosition;
    fn measured(&self) -> Option<MeasuredDimensions>;
    fn raw_width(&self) -> Option<f64>;
    fn raw_height(&self) -> Option<f64>;
    fn initial_width(&self) -> Option<f64>;
    fn initial_height(&self) -> Option<f64>;
    fn origin(&self) -> Option<NodeOrigin>;
}

impl<D: Clone> NodeLike for Node<D> {
    fn id(&self) -> &str {
        &self.id
    }
    fn position(&self) -> XYPosition {
        self.position
    }
    fn measured(&self) -> Option<MeasuredDimensions> {
        self.measured
    }
    fn raw_width(&self) -> Option<f64> {
        self.width
    }
    fn raw_height(&self) -> Option<f64> {
        self.height
    }
    fn initial_width(&self) -> Option<f64> {
        self.initial_width
    }
    fn initial_height(&self) -> Option<f64> {
        self.initial_height
    }
    fn origin(&self) -> Option<NodeOrigin> {
        self.origin
    }
}

impl<D: Clone> NodeLike for InternalNode<D> {
    fn id(&self) -> &str {
        &self.user.id
    }
    fn position(&self) -> XYPosition {
        self.user.position
    }
    fn measured(&self) -> Option<MeasuredDimensions> {
        Some(self.measured)
    }
    fn raw_width(&self) -> Option<f64> {
        self.user.width
    }
    fn raw_height(&self) -> Option<f64> {
        self.user.height
    }
    fn initial_width(&self) -> Option<f64> {
        self.user.initial_width
    }
    fn initial_height(&self) -> Option<f64> {
        self.user.initial_height
    }
    fn origin(&self) -> Option<NodeOrigin> {
        self.user.origin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_minimal_constructor() {
        let n: Node<()> = Node::minimal("n1", 10.0, 20.0);
        assert_eq!(n.id, "n1");
        assert_eq!(n.position, XYPosition::new(10.0, 20.0));
        assert!(matches!(n.extent, NodeExtent::Unbounded));
    }

    #[test]
    fn internal_node_from_user_preserves_position() {
        let n: Node<()> = Node::minimal("n1", 5.0, 7.0);
        let internal = InternalNode::from_user(n);
        assert_eq!(internal.user.id, "n1");
        assert_eq!(internal.internals.position_absolute, XYPosition::new(5.0, 7.0));
    }

    #[test]
    fn node_extent_default_is_unbounded() {
        assert!(matches!(NodeExtent::default(), NodeExtent::Unbounded));
    }

    #[test]
    fn node_like_works_for_both() {
        let n: Node<()> = Node::minimal("n1", 1.0, 2.0);
        let i = InternalNode::from_user(n.clone());
        let n_like: &dyn NodeLike = &n;
        let i_like: &dyn NodeLike = &i;
        assert_eq!(n_like.id(), "n1");
        assert_eq!(i_like.id(), "n1");
        assert_eq!(n_like.position(), i_like.position());
    }

    #[test]
    fn align_default_is_center() {
        assert_eq!(Align::default(), Align::Center);
    }
}
