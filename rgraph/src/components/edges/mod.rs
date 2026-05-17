//! Port of `xyflow-react/src/components/Edges/`.
//!
//! Status: Phase 6 — implemented.

pub mod base_edge;
pub mod bezier_edge;
pub mod edge_anchor;
pub mod edge_text;
pub mod simple_bezier_edge;
pub mod smoothstep_edge;
pub mod step_edge;
pub mod straight_edge;

pub use base_edge::{BaseEdge, BaseEdgeComponentProps};
pub use bezier_edge::{BezierEdge, BezierEdgeComponentProps};
pub use edge_anchor::{EdgeAnchor, EdgeAnchorProps};
pub use edge_text::{EdgeText, EdgeTextProps};
pub use simple_bezier_edge::{
    get_simple_bezier_path, GetSimpleBezierPathParams, SimpleBezierEdge,
    SimpleBezierEdgeComponentProps,
};
pub use smoothstep_edge::{SmoothStepEdge, SmoothStepEdgeComponentProps};
pub use step_edge::{StepEdge, StepEdgeComponentProps};
pub use straight_edge::{StraightEdge, StraightEdgeComponentProps};
