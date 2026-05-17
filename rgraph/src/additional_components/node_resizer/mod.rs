//! Port of `xyflow-react/src/additional-components/NodeResizer/`.
//!
//! Status: Phase 8 — implemented (visual chrome). Interactive resize
//! pointer plumbing deferred.

pub mod node_resize_control;
pub mod node_resizer;
pub mod types;

pub use node_resize_control::{NodeResizeControl, NodeResizeControlProps, ResizeControlLine};
pub use node_resizer::{NodeResizer, NodeResizerProps};
pub use types::{
    control_position_classes, default_position_for, NodeResizerCommon,
    NodeResizerControlPosition, NodeResizerDirection, NodeResizerVariant,
};
