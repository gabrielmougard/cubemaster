//! Port of `xyflow-react/src/components/NodeWrapper/utils.tsx`.
//!
//! Status: Phase 5 — implemented.
//!
//! * [`arrow_key_diff`] — maps arrow-key names (`ArrowUp`, …) to the
//!   unit-direction vector used by [`crate::hooks::use_move_selected_nodes`].
//! * [`built_in_node_renderer`] — resolves a built-in node type name
//!   to its renderer component. Mirrors the TS `builtinNodeTypes`
//!   record.
//! * [`get_node_inline_style_dimensions`] — picks the appropriate
//!   `(width, height)` inline-style values for a given internal node.

#![allow(clippy::module_name_repetitions)]

use rgraph_core::types::geometry::XYPosition;

use crate::types::nodes::InternalNode;

/// Returns the `XYPosition` direction for one of the four arrow keys,
/// or `None` for any other key. Mirrors the TS `arrowKeyDiffs` record.
#[must_use]
pub fn arrow_key_diff(key: &str) -> Option<XYPosition> {
    match key {
        "ArrowUp" => Some(XYPosition::new(0.0, -1.0)),
        "ArrowDown" => Some(XYPosition::new(0.0, 1.0)),
        "ArrowLeft" => Some(XYPosition::new(-1.0, 0.0)),
        "ArrowRight" => Some(XYPosition::new(1.0, 0.0)),
        _ => None,
    }
}

/// Identifier for one of the four built-in node renderers.
///
/// Mirrors the TS `builtinNodeTypes` keys. We use an enum (instead of
/// a string-keyed `HashMap`) so the match is exhaustive at compile
/// time. The string spelling matches the TS source exactly via
/// [`BuiltInNodeType::from_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInNodeType {
    Input,
    Default,
    Output,
    Group,
}

impl BuiltInNodeType {
    /// Parse from the TS-style string identifier.
    ///
    /// Named `parse` (not `from_str`) to avoid colliding with
    /// `std::str::FromStr::from_str`, which would change the return
    /// type to `Result<Self, _>`.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "input" => Some(BuiltInNodeType::Input),
            "default" => Some(BuiltInNodeType::Default),
            "output" => Some(BuiltInNodeType::Output),
            "group" => Some(BuiltInNodeType::Group),
            _ => None,
        }
    }

    /// The TS-style string identifier (`"input"`, `"default"`, …).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BuiltInNodeType::Input => "input",
            BuiltInNodeType::Default => "default",
            BuiltInNodeType::Output => "output",
            BuiltInNodeType::Group => "group",
        }
    }
}

/// Width / height pair returned by [`get_node_inline_style_dimensions`].
///
/// Both fields are `None` when the corresponding dimension hasn't
/// been measured or supplied. The TS source returns `string | number |
/// undefined`; we collapse to `Option<f64>` since CSS pixel values
/// are sufficient for the Dioxus port.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InlineDimensions {
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// Pick the appropriate `(width, height)` inline-style values for
/// `node`.
///
/// Mirrors the TS `getNodeInlineStyleDimensions` (lines 23–39 of
/// `NodeWrapper/utils.tsx`). When the node has not yet had its
/// handle bounds measured, the TS source falls back to
/// `node.initialWidth/initialHeight` so the layout reserves space.
/// After measurement, only `node.width/height` are honoured (the
/// initial fallback is dropped).
#[must_use]
pub fn get_node_inline_style_dimensions<D: Clone>(
    node: &InternalNode<D>,
) -> InlineDimensions {
    if node.internals.handle_bounds.is_none() {
        InlineDimensions {
            width: node.user.width.or(node.user.initial_width),
            height: node.user.height.or(node.user.initial_height),
        }
    } else {
        InlineDimensions {
            width: node.user.width,
            height: node.user.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::nodes::Node;
    use rgraph_core::types::nodes::{InternalNode as CoreInternalNode, MeasuredDimensions};

    #[test]
    fn arrow_key_diff_known_keys() {
        assert_eq!(arrow_key_diff("ArrowUp"), Some(XYPosition::new(0.0, -1.0)));
        assert_eq!(arrow_key_diff("ArrowRight"), Some(XYPosition::new(1.0, 0.0)));
        assert!(arrow_key_diff("Space").is_none());
    }

    #[test]
    fn built_in_node_type_round_trip() {
        for s in ["input", "default", "output", "group"] {
            let t = BuiltInNodeType::parse(s).unwrap();
            assert_eq!(t.as_str(), s);
        }
        assert!(BuiltInNodeType::parse("custom").is_none());
    }

    #[test]
    fn inline_dims_use_initial_fallback_before_measurement() {
        let mut n = Node::<()>::minimal("n1", 0.0, 0.0);
        n.initial_width = Some(100.0);
        n.initial_height = Some(80.0);
        let internal = CoreInternalNode::from_user(n);
        let d = get_node_inline_style_dimensions(&internal);
        assert_eq!(d.width, Some(100.0));
        assert_eq!(d.height, Some(80.0));
    }

    #[test]
    fn inline_dims_drop_initial_after_handle_bounds_set() {
        let mut n = Node::<()>::minimal("n1", 0.0, 0.0);
        n.initial_width = Some(100.0);
        n.initial_height = Some(80.0);
        let mut internal = CoreInternalNode::from_user(n);
        internal.measured = MeasuredDimensions {
            width: Some(50.0),
            height: Some(40.0),
        };
        internal.internals.handle_bounds = Some(rgraph_core::types::nodes::NodeHandleBounds::default());
        let d = get_node_inline_style_dimensions(&internal);
        // After measurement, only `user.width/height` are returned —
        // both are `None` because we set `measured` rather than the
        // user fields. That matches the TS behaviour (the inline
        // styles only forward the user-supplied width/height once
        // handles are bound; layout uses measured separately).
        assert_eq!(d.width, None);
        assert_eq!(d.height, None);
    }
}
