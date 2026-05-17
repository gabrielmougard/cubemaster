//! Port of `xyflow-react/src/additional-components/`.
//!
//! Status: Phase 8 — implemented. These ship as opt-in components in
//! the original library. Interactive plumbing for [`MiniMap`] (pan/
//! zoom) and [`NodeResizer`] (pointer-driven resize) is intentionally
//! deferred and tracked in each module's TODO header.

pub mod background;
pub mod controls;
pub mod edge_toolbar;
pub mod minimap;
pub mod node_resizer;
pub mod node_toolbar;

pub use background::{
    Background, BackgroundGap, BackgroundOffset, BackgroundProps, BackgroundVariant, DotPattern,
    LinePattern,
};
pub use controls::{
    ControlButton, ControlButtonProps, Controls, ControlsFitViewOptions, ControlsOrientation,
    ControlsProps, FitViewIcon, LockIcon, MinusIcon, PlusIcon, UnlockIcon,
};
pub use edge_toolbar::{EdgeToolbar, EdgeToolbarProps};
pub use minimap::{MiniMap, MiniMapNode, MiniMapNodeAttr, MiniMapNodes, MiniMapProps};
pub use node_resizer::{
    NodeResizeControl, NodeResizeControlProps, NodeResizer, NodeResizerCommon,
    NodeResizerControlPosition, NodeResizerDirection, NodeResizerProps, NodeResizerVariant,
    ResizeControlLine,
};
pub use node_toolbar::{
    NodeToolbar, NodeToolbarPortal, NodeToolbarProps, NodeToolbarTarget,
    DEFAULT_TOOLBAR_ALIGN, DEFAULT_TOOLBAR_OFFSET, DEFAULT_TOOLBAR_POSITION,
};
