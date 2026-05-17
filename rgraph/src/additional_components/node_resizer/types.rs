//! Port of `xyflow-react/src/additional-components/NodeResizer/types.ts`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

use rgraph_core::xyresizer::types::{
    ControlPosition, OnResizeEndFn, OnResizeFn, OnResizeStartFn, ResizeControlDirection,
    ResizeControlVariant, ShouldResizeFn,
};

/// Common props shared between [`super::node_resizer::NodeResizer`] and
/// [`super::node_resize_control::NodeResizeControl`].
#[derive(Clone)]
pub struct NodeResizerCommon {
    pub node_id: Option<String>,
    pub color: Option<String>,
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,
    pub keep_aspect_ratio: bool,
    pub auto_scale: bool,
    pub should_resize: Option<ShouldResizeFn>,
    pub on_resize_start: Option<OnResizeStartFn>,
    pub on_resize: Option<OnResizeFn>,
    pub on_resize_end: Option<OnResizeEndFn>,
}

impl Default for NodeResizerCommon {
    fn default() -> Self {
        Self {
            node_id: None,
            color: None,
            min_width: 10.0,
            min_height: 10.0,
            max_width: f64::MAX,
            max_height: f64::MAX,
            keep_aspect_ratio: false,
            auto_scale: true,
            should_resize: None,
            on_resize_start: None,
            on_resize: None,
            on_resize_end: None,
        }
    }
}

impl PartialEq for NodeResizerCommon {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
            && self.color == other.color
            && self.min_width == other.min_width
            && self.min_height == other.min_height
            && self.max_width == other.max_width
            && self.max_height == other.max_height
            && self.keep_aspect_ratio == other.keep_aspect_ratio
            && self.auto_scale == other.auto_scale
    }
}

/// CSS class names derived from a [`ControlPosition`], matching the TS
/// `position.split('-')` behaviour.
#[must_use]
pub fn control_position_classes(pos: ControlPosition) -> &'static [&'static str] {
    match pos {
        ControlPosition::Top => &["top"],
        ControlPosition::Bottom => &["bottom"],
        ControlPosition::Left => &["left"],
        ControlPosition::Right => &["right"],
        ControlPosition::TopLeft => &["top", "left"],
        ControlPosition::TopRight => &["top", "right"],
        ControlPosition::BottomLeft => &["bottom", "left"],
        ControlPosition::BottomRight => &["bottom", "right"],
    }
}

/// `ResizeControlVariant::Handle` → default position bottom-right.
/// `ResizeControlVariant::Line` → default position right.
#[must_use]
pub fn default_position_for(variant: ResizeControlVariant) -> ControlPosition {
    match variant {
        ResizeControlVariant::Line => ControlPosition::Right,
        ResizeControlVariant::Handle => ControlPosition::BottomRight,
    }
}

pub use rgraph_core::xyresizer::types::{
    ControlPosition as NodeResizerControlPosition, ResizeControlDirection as NodeResizerDirection,
    ResizeControlVariant as NodeResizerVariant,
};
// Suppress "unused import" warnings when only some re-exports are
// consumed downstream.
#[allow(dead_code)]
fn _ensure_imports() {
    let _ = std::marker::PhantomData::<(ResizeControlDirection,)>;
}
