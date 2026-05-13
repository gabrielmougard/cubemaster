//! Port of `xyflow-core/src/types/handles.ts`.
//!
//! Status: implemented (phase 1).

#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::connection::IsValidConnection;
use crate::types::geometry::Position;

/// Whether a handle initiates a connection (`Source`) or receives one
/// (`Target`).
///
/// Mirrors the TS string literal union `'source' | 'target'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum HandleType {
    Source,
    Target,
}

impl HandleType {
    /// Returns the opposite handle type — `Source` ↔ `Target`. Useful
    /// when looking up the matching handle on the other side of a
    /// candidate edge.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            HandleType::Source => HandleType::Target,
            HandleType::Target => HandleType::Source,
        }
    }
}

/// Concrete, measured handle attached to a node.
///
/// Created internally by xyflow when a node is mounted; rarely
/// constructed by user code.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Handle {
    /// Optional handle id. `None` (or JS `null`) means the node has a
    /// single handle of this [`HandleType`] and the id is implicit.
    #[cfg_attr(feature = "serde", serde(default))]
    pub id: Option<String>,
    /// Id of the node that owns this handle.
    pub node_id: String,
    pub x: f64,
    pub y: f64,
    pub position: Position,
    /// `Source` or `Target`. `type` is reserved in Rust, so we use
    /// `type_` for parity with similar ports in the workspace.
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub type_: HandleType,
    pub width: f64,
    pub height: f64,
}

/// User-facing handle props, used by Dioxus components when declaring a
/// handle inside a custom node.
///
/// Mirrors the TS `HandleProps`. `is_valid_connection` is stored as a
/// [`IsValidConnection`] (boxed predicate) and is therefore not
/// `Clone`/`PartialEq`.
pub struct HandleProps {
    /// `Source` or `Target`. Default in TS is `"source"`.
    pub type_: HandleType,
    /// Side of the node the handle sits on. TS default is `Position::Top`.
    pub position: Position,
    /// Master toggle. Defaults to `true`.
    pub is_connectable: Option<bool>,
    /// Whether a connection can *start* at this handle. Defaults to `true`.
    pub is_connectable_start: Option<bool>,
    /// Whether a connection can *end* at this handle. Defaults to `true`.
    pub is_connectable_end: Option<bool>,
    /// Optional per-handle validation. The doc comment in the TS source
    /// recommends moving this to the top-level `isValidConnection` for
    /// performance reasons; kept here for parity.
    pub is_valid_connection: Option<IsValidConnection>,
    /// Handle id. Optional when only one handle of this type exists on
    /// the node.
    pub id: Option<String>,
}

impl std::fmt::Debug for HandleProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandleProps")
            .field("type_", &self.type_)
            .field("position", &self.position)
            .field("is_connectable", &self.is_connectable)
            .field("is_connectable_start", &self.is_connectable_start)
            .field("is_connectable_end", &self.is_connectable_end)
            .field(
                "is_valid_connection",
                &self.is_valid_connection.as_ref().map(|_| "<fn>"),
            )
            .field("id", &self.id)
            .finish()
    }
}

impl Default for HandleProps {
    fn default() -> Self {
        HandleProps {
            type_: HandleType::Source,
            position: Position::Top,
            is_connectable: None,
            is_connectable_start: None,
            is_connectable_end: None,
            is_valid_connection: None,
            id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_type_opposite_round_trip() {
        assert_eq!(HandleType::Source.opposite(), HandleType::Target);
        assert_eq!(HandleType::Target.opposite(), HandleType::Source);
        assert_eq!(HandleType::Source.opposite().opposite(), HandleType::Source);
    }

    #[test]
    fn handle_props_default() {
        let p = HandleProps::default();
        assert_eq!(p.type_, HandleType::Source);
        assert_eq!(p.position, Position::Top);
        assert!(p.is_connectable.is_none());
    }
}
