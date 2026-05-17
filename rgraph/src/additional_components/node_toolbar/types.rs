//! Port of `xyflow-react/src/additional-components/NodeToolbar/types.ts`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use rgraph_core::types::geometry::Position;
use rgraph_core::types::nodes::Align;

/// Mirrors the TS `NodeToolbarProps['nodeId']: string | string[]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeToolbarTarget {
    Single(String),
    Many(Vec<String>),
}

impl NodeToolbarTarget {
    pub fn ids(&self) -> Vec<String> {
        match self {
            NodeToolbarTarget::Single(s) => vec![s.clone()],
            NodeToolbarTarget::Many(v) => v.clone(),
        }
    }
}

impl From<&str> for NodeToolbarTarget {
    fn from(s: &str) -> Self {
        NodeToolbarTarget::Single(s.to_string())
    }
}

impl From<String> for NodeToolbarTarget {
    fn from(s: String) -> Self {
        NodeToolbarTarget::Single(s)
    }
}

impl From<Vec<String>> for NodeToolbarTarget {
    fn from(v: Vec<String>) -> Self {
        NodeToolbarTarget::Many(v)
    }
}

/// Defaults for `NodeToolbar`. Mirror the TS prop defaults.
pub const DEFAULT_TOOLBAR_POSITION: Position = Position::Top;
pub const DEFAULT_TOOLBAR_ALIGN: Align = Align::Center;
pub const DEFAULT_TOOLBAR_OFFSET: f64 = 10.0;
