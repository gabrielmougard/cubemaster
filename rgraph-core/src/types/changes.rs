//! Port of `xyflow-core/src/types/changes.ts`.
//!
//! Status: implemented (phase 1).
//!
//! `NodeChange` and `EdgeChange` are tagged unions in TS; in Rust we
//! model them as enums. The TS `setAttributes: boolean | 'width' | 'height'`
//! tri-state becomes [`SetAttributesMode`].

#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::edges::Edge;
use crate::types::geometry::{Dimensions, XYPosition};
use crate::types::nodes::Node;

/// Tri-state controlling whether a `NodeChange::Dimensions` should also
/// write to the node's `width` / `height` fields (not only `measured`).
///
/// Mirrors the TS `boolean | 'width' | 'height'`:
/// * `false`     → `None`,
/// * `true`      → `All`,
/// * `'width'`   → `WidthOnly`,
/// * `'height'`  → `HeightOnly`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum SetAttributesMode {
    #[default]
    None,
    All,
    WidthOnly,
    HeightOnly,
}

/// Update produced by the renderer / store and passed to the user via
/// `on_nodes_change`.
///
/// Mirrors the TS union of six `NodeXxxChange` types. Each variant
/// carries the same data shape as its TS counterpart, with serde
/// `tag = "type"` so wire format matches the JS exactly.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "lowercase"))]
pub enum NodeChange<D: Clone = ()> {
    /// Measurement update.
    Dimensions {
        id: String,
        #[cfg_attr(feature = "serde", serde(default))]
        dimensions: Option<Dimensions>,
        /// True while a NodeResizer is actively resizing.
        #[cfg_attr(feature = "serde", serde(default))]
        resizing: Option<bool>,
        /// Should the dimensions be written to the user node's
        /// `width`/`height` fields, or only to `measured`?
        #[cfg_attr(feature = "serde", serde(default))]
        set_attributes: SetAttributesMode,
    },
    /// Position update from a drag or programmatic move.
    Position {
        id: String,
        #[cfg_attr(feature = "serde", serde(default))]
        position: Option<XYPosition>,
        #[cfg_attr(feature = "serde", serde(default))]
        position_absolute: Option<XYPosition>,
        #[cfg_attr(feature = "serde", serde(default))]
        dragging: Option<bool>,
    },
    /// Selection toggle.
    #[cfg_attr(feature = "serde", serde(rename = "select"))]
    Select { id: String, selected: bool },
    /// Removal.
    Remove { id: String },
    /// New node.
    Add {
        item: Node<D>,
        #[cfg_attr(feature = "serde", serde(default))]
        index: Option<usize>,
    },
    /// In-place replace.
    Replace { id: String, item: Node<D> },
}

/// Edge counterpart of [`NodeChange`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "lowercase"))]
pub enum EdgeChange<D: Clone = ()> {
    /// Selection toggle.
    #[cfg_attr(feature = "serde", serde(rename = "select"))]
    Select { id: String, selected: bool },
    /// Removal.
    Remove { id: String },
    /// New edge.
    Add {
        item: Edge<D>,
        #[cfg_attr(feature = "serde", serde(default))]
        index: Option<usize>,
    },
    /// In-place replace.
    Replace { id: String, item: Edge<D> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_attributes_mode_default_is_none() {
        assert_eq!(SetAttributesMode::default(), SetAttributesMode::None);
    }

    #[test]
    fn node_change_variants_construct() {
        let _c: NodeChange<()> = NodeChange::Position {
            id: "n1".into(),
            position: Some(XYPosition::new(1.0, 2.0)),
            position_absolute: None,
            dragging: Some(true),
        };
        let _c2: NodeChange<()> = NodeChange::Remove { id: "n1".into() };
        let _c3: NodeChange<()> = NodeChange::Select {
            id: "n1".into(),
            selected: true,
        };
    }

    #[test]
    fn edge_change_variants_construct() {
        let e = Edge::<()>::minimal("e1", "a", "b");
        let _c: EdgeChange<()> = EdgeChange::Add {
            item: e,
            index: Some(0),
        };
    }
}
