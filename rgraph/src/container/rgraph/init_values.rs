//! Port of `xyflow-react/src/container/ReactFlow/init-values.ts`.
//!
//! Status: Phase 7 — implemented.
//!
//! Default values referenced by [`crate::container::rgraph::RGraph`]
//! and its descendants.

#![allow(clippy::module_name_repetitions)]

use rgraph_core::types::nodes::NodeOrigin;
use rgraph_core::types::viewport::Viewport;

/// Default node origin (`[0, 0]` — top-left).
pub const DEFAULT_NODE_ORIGIN: NodeOrigin = (0.0, 0.0);

/// Default viewport (identity).
pub const DEFAULT_VIEWPORT: Viewport = Viewport {
    x: 0.0,
    y: 0.0,
    zoom: 1.0,
};
