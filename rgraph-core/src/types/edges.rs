//! Port of `xyflow-core/src/types/edges.ts`.
//!
//! Status: implemented (phase 1).

#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::geometry::Position;

/// Edge data structure, generic over user data.
///
/// Mirrors the TS `EdgeBase<EdgeData, EdgeType>`. The TS `type` field
/// is renamed to `type_` to avoid clashing with the Rust keyword
/// (serialised as `"type"` under the `serde` feature).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Edge<D: Clone = ()> {
    /// Unique id of the edge.
    pub id: String,
    /// Type of edge defined in `edgeTypes`.
    #[cfg_attr(feature = "serde", serde(rename = "type", default))]
    pub type_: Option<String>,
    /// Id of source node.
    pub source: String,
    /// Id of target node.
    pub target: String,
    /// Id of source handle, only needed if there are multiple handles
    /// per node.
    #[cfg_attr(feature = "serde", serde(default))]
    pub source_handle: Option<String>,
    /// Id of target handle, only needed if there are multiple handles
    /// per node.
    #[cfg_attr(feature = "serde", serde(default))]
    pub target_handle: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub animated: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub hidden: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub deletable: Option<bool>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selectable: Option<bool>,
    /// Arbitrary user data.
    pub data: D,
    #[cfg_attr(feature = "serde", serde(default))]
    pub selected: Option<bool>,
    /// Marker on the beginning of the edge.
    #[cfg_attr(feature = "serde", serde(default))]
    pub marker_start: Option<EdgeMarkerType>,
    /// Marker on the end of the edge.
    #[cfg_attr(feature = "serde", serde(default))]
    pub marker_end: Option<EdgeMarkerType>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub z_index: Option<i32>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub aria_label: Option<String>,
    /// Width of the invisible interaction path drawn around each edge.
    #[cfg_attr(feature = "serde", serde(default))]
    pub interaction_width: Option<f64>,
}

impl<D: Clone + Default> Edge<D> {
    /// Helper that produces a minimal edge from an id and a
    /// `(source, target)` pair, leaving everything else at the type's
    /// default. Useful in tests.
    #[must_use]
    pub fn minimal(id: impl Into<String>, source: impl Into<String>, target: impl Into<String>) -> Self {
        Edge {
            id: id.into(),
            type_: None,
            source: source.into(),
            target: target.into(),
            source_handle: None,
            target_handle: None,
            animated: None,
            hidden: None,
            deletable: None,
            selectable: None,
            data: D::default(),
            selected: None,
            marker_start: None,
            marker_end: None,
            z_index: None,
            aria_label: None,
            interaction_width: None,
        }
    }
}

/// Path-tuning options for the smoothstep edge variant.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SmoothStepPathOptions {
    pub offset: Option<f64>,
    pub border_radius: Option<f64>,
    pub step_position: Option<f64>,
}

/// Path-tuning options for the step edge variant.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StepPathOptions {
    pub offset: Option<f64>,
}

/// Path-tuning options for the bezier edge variant.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BezierPathOptions {
    pub curvature: Option<f64>,
}

/// Style of connection line drawn while a new edge is being created.
///
/// The serde renames mirror the JS string values exactly, including the
/// quirky `Bezier = "default"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ConnectionLineType {
    #[cfg_attr(feature = "serde", serde(rename = "default"))]
    Bezier,
    #[cfg_attr(feature = "serde", serde(rename = "straight"))]
    Straight,
    #[cfg_attr(feature = "serde", serde(rename = "step"))]
    Step,
    #[cfg_attr(feature = "serde", serde(rename = "smoothstep"))]
    SmoothStep,
    #[cfg_attr(feature = "serde", serde(rename = "simplebezier"))]
    SimpleBezier,
}

impl Default for ConnectionLineType {
    fn default() -> Self {
        Self::Bezier
    }
}

/// Built-in marker shapes available without supplying a full
/// [`EdgeMarker`] config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum MarkerType {
    Arrow,
    ArrowClosed,
}

/// Full marker configuration for an edge end-cap.
///
/// Mirrors the TS `EdgeMarker`. The TS `type` field is renamed to
/// `type_` for the same reason as on [`Edge`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeMarker {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub type_: MarkerType,
    #[cfg_attr(feature = "serde", serde(default))]
    pub color: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub width: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub height: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub marker_units: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub orient: Option<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub stroke_width: Option<f64>,
}

/// Either a custom marker config or a string id pointing at a
/// pre-registered marker (the TS `string | EdgeMarker` union).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum EdgeMarkerType {
    /// Reference to a pre-registered marker by id.
    Builtin(String),
    /// Inline marker config.
    Custom(EdgeMarker),
}

/// Concrete marker render-props with the resolved DOM id, used by the
/// `<MarkerDefinitions />` component in the React/Svelte ports.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MarkerProps {
    pub id: String,
    pub marker: EdgeMarker,
}

/// Pre-computed edge endpoint positions.
///
/// Mirrors the TS `EdgePosition`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgePosition {
    pub source_x: f64,
    pub source_y: f64,
    pub target_x: f64,
    pub target_y: f64,
    pub source_position: Position,
    pub target_position: Position,
}

/// `id -> Edge<D>` lookup mirroring the TS `EdgeLookup`.
pub type EdgeLookup<D = ()> = HashMap<String, Edge<D>>;

/// Horizontal alignment of the edge toolbar (`left`, `center`, `right`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum AlignX {
    Left,
    #[default]
    Center,
    Right,
}

/// Vertical alignment of the edge toolbar (`top`, `center`, `bottom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum AlignY {
    Top,
    #[default]
    Center,
    Bottom,
}

/// Render props for the edge toolbar component.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeToolbarBaseProps {
    pub x: f64,
    pub y: f64,
    /// If `true`, the toolbar is visible even when the edge is not
    /// selected. Defaults to `false`.
    pub is_visible: Option<bool>,
    pub align_x: Option<AlignX>,
    pub align_y: Option<AlignY>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_minimal_constructor() {
        let e: Edge<()> = Edge::minimal("e1", "n1", "n2");
        assert_eq!(e.id, "e1");
        assert_eq!(e.source, "n1");
        assert_eq!(e.target, "n2");
        assert!(e.type_.is_none());
        assert!(e.marker_start.is_none());
    }

    #[test]
    fn connection_line_default_is_bezier() {
        assert_eq!(ConnectionLineType::default(), ConnectionLineType::Bezier);
    }

    #[test]
    fn align_defaults_center() {
        assert_eq!(AlignX::default(), AlignX::Center);
        assert_eq!(AlignY::default(), AlignY::Center);
    }

    #[test]
    fn edge_marker_types_are_distinct() {
        let a = EdgeMarkerType::Builtin("foo".into());
        let b = EdgeMarkerType::Custom(EdgeMarker {
            type_: MarkerType::Arrow,
            color: None,
            width: None,
            height: None,
            marker_units: None,
            orient: None,
            stroke_width: None,
        });
        assert_ne!(a, b);
    }
}
