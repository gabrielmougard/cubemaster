//! Port of `xyflow-core/src/xyresizer/types.ts`.
//!
//! Status: implemented (phase 7).

#![allow(clippy::module_name_repetitions)]

use std::rc::Rc;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::nodes::PointerEventLike;

/// Position-and-size payload reported to `on_resize_start` /
/// `on_resize_end` callbacks. Mirrors the TS `ResizeParams`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResizeParams {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Resize axis direction returned by [`crate::xyresizer::utils::get_resize_direction`].
///
/// Each axis is one of `-1`, `0`, `+1`. Used by user code in
/// `on_resize` / `should_resize` to detect which way the resize is
/// growing on each axis (sign depends on which control was grabbed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResizeDirection {
    pub x: i8,
    pub y: i8,
}

impl ResizeDirection {
    /// Convert to the `[x, y]` array shape TS uses.
    #[must_use]
    pub const fn as_array(&self) -> [i8; 2] {
        [self.x, self.y]
    }
}

/// `ResizeParams` plus the per-axis growth direction. Mirrors the TS
/// `ResizeParamsWithDirection`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResizeParamsWithDirection {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub direction: ResizeDirection,
}

/// Which side of the node's bounding box the user is dragging.
///
/// Mirrors the TS `ControlPosition`. The four `…Corner` variants are
/// also used as `ControlLinePosition` in TS — Rust collapses them
/// into a single enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum ControlPosition {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ControlPosition {
    /// Returns `true` when the control affects the *horizontal* axis
    /// (i.e. has "left" or "right" component).
    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(
            self,
            ControlPosition::Left
                | ControlPosition::Right
                | ControlPosition::TopLeft
                | ControlPosition::TopRight
                | ControlPosition::BottomLeft
                | ControlPosition::BottomRight
        )
    }

    /// Returns `true` when the control affects the *vertical* axis
    /// (i.e. has "top" or "bottom" component).
    #[must_use]
    pub const fn is_vertical(self) -> bool {
        matches!(
            self,
            ControlPosition::Top
                | ControlPosition::Bottom
                | ControlPosition::TopLeft
                | ControlPosition::TopRight
                | ControlPosition::BottomLeft
                | ControlPosition::BottomRight
        )
    }

    /// `true` when dragging this control affects the `x` field of the
    /// node (i.e. the control is on the *left* side).
    #[must_use]
    pub const fn affects_x(self) -> bool {
        matches!(
            self,
            ControlPosition::Left | ControlPosition::TopLeft | ControlPosition::BottomLeft
        )
    }

    /// `true` when dragging this control affects the `y` field of the
    /// node (i.e. the control is on the *top* side).
    #[must_use]
    pub const fn affects_y(self) -> bool {
        matches!(
            self,
            ControlPosition::Top | ControlPosition::TopLeft | ControlPosition::TopRight
        )
    }
}

/// Result of [`crate::xyresizer::utils::get_control_direction`]. Mirrors
/// the inline TS return type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlDirection {
    pub is_horizontal: bool,
    pub is_vertical: bool,
    pub affects_x: bool,
    pub affects_y: bool,
}

/// Variant of the resize control. Mirrors the TS enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ResizeControlVariant {
    Line,
    Handle,
}

/// Axis the user can resize the node on (used by Line controls).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ResizeControlDirection {
    Horizontal,
    Vertical,
}

/// Four corner positions, mirrors the TS `XY_RESIZER_HANDLE_POSITIONS`.
pub const XY_RESIZER_HANDLE_POSITIONS: [ControlPosition; 4] = [
    ControlPosition::TopLeft,
    ControlPosition::TopRight,
    ControlPosition::BottomLeft,
    ControlPosition::BottomRight,
];

/// Four side positions, mirrors the TS `XY_RESIZER_LINE_POSITIONS`.
pub const XY_RESIZER_LINE_POSITIONS: [ControlPosition; 4] = [
    ControlPosition::Top,
    ControlPosition::Right,
    ControlPosition::Bottom,
    ControlPosition::Left,
];

// ---------------------------------------------------------------------------
// Callback aliases
// ---------------------------------------------------------------------------

/// Callback to determine whether the node should actually resize.
///
/// Mirrors the TS `ShouldResize`. Returning `false` cancels the
/// pending resize tick.
pub type ShouldResizeFn = Rc<dyn Fn(&PointerEventLike, &ResizeParamsWithDirection) -> bool>;

/// Mirrors the TS `OnResizeStart`.
pub type OnResizeStartFn = Rc<dyn Fn(&PointerEventLike, &ResizeParams)>;

/// Mirrors the TS `OnResize`.
pub type OnResizeFn = Rc<dyn Fn(&PointerEventLike, &ResizeParamsWithDirection)>;

/// Mirrors the TS `OnResizeEnd`.
pub type OnResizeEndFn = Rc<dyn Fn(&PointerEventLike, &ResizeParams)>;

/// Boundaries clamp the resize result. Mirrors the TS inline shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResizeBoundaries {
    pub min_width: f64,
    pub min_height: f64,
    pub max_width: f64,
    pub max_height: f64,
}

impl Default for ResizeBoundaries {
    fn default() -> Self {
        Self {
            min_width: 0.0,
            min_height: 0.0,
            max_width: f64::MAX,
            max_height: f64::MAX,
        }
    }
}

/// Snapshot of the previous resize tick's geometry. Used internally;
/// public so consumers can persist it across `update` reconfigurations
/// if they want continuity.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PrevValues {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
}

/// Snapshot taken at the start of a resize gesture. Mirrors the TS
/// inline `StartValues`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StartValues {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    pub pointer_x: f64,
    pub pointer_y: f64,
    pub aspect_ratio: f64,
}

/// Per-tick result emitted by the resizer (TS `XYResizerChange` — every
/// field is `Option<f64>` so the consumer can detect what actually
/// changed this tick).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResizerChange {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// Adjustment applied to a child node when the parent is resized
/// (TS `XYResizerChildChange`).
#[derive(Debug, Clone, PartialEq)]
pub struct ResizerChildChange {
    pub id: String,
    pub position: crate::types::geometry::XYPosition,
    pub extent: crate::types::nodes::NodeExtent,
}
