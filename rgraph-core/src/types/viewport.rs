//! Port of viewport-related types from `xyflow-core/src/types/general.ts`.
//!
//! Status: implemented (phase 1).
//!
//! Covers `Viewport`, panel positions, scroll modes, padding parsing
//! enums, viewport-helper option structs, color modes, z-index modes,
//! and a few small boxed callback aliases used in the public API.

#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::types::geometry::Rect;

/// Internally, the flow maintains a coordinate system that is
/// independent of the rest of the page. The [`Viewport`] tells you
/// where in that system the flow is currently displayed and how
/// zoomed in or out it is.
///
/// A [`crate::types::geometry::Transform`] has the same numeric
/// content as the viewport, but represents the inverse mapping; do
/// not confuse them.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Viewport {
    /// `Viewport { x: 0.0, y: 0.0, zoom: 1.0 }`.
    pub const IDENTITY: Viewport = Viewport {
        x: 0.0,
        y: 0.0,
        zoom: 1.0,
    };

    #[inline]
    #[must_use]
    pub const fn new(x: f64, y: f64, zoom: f64) -> Self {
        Viewport { x, y, zoom }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Modes for how the viewport pans when the user scrolls.
///
/// * [`PanOnScrollMode::Free`]      — pan in any direction.
/// * [`PanOnScrollMode::Vertical`]  — restrict to the vertical axis.
/// * [`PanOnScrollMode::Horizontal`] — restrict to the horizontal axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PanOnScrollMode {
    Free,
    Vertical,
    Horizontal,
}

impl Default for PanOnScrollMode {
    fn default() -> Self {
        Self::Free
    }
}

/// User-selection rectangle behaviour.
///
/// `Partial` selects nodes that overlap the rect; `Full` selects only
/// nodes that are entirely contained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum SelectionMode {
    Partial,
    Full,
}

impl Default for SelectionMode {
    fn default() -> Self {
        Self::Full
    }
}

/// Easing function used by animated viewport transitions, mirroring the
/// JS `(t: number) => number` callback.
///
/// Stored boxed because the value is set on user-supplied option
/// structs.
pub type EaseFn = Box<dyn Fn(f64) -> f64 + Send + Sync>;

/// Choice between the two interpolation strategies offered by xyflow:
/// [`InterpolationKind::Smooth`] uses van Wijk smooth-zoom (the d3
/// default), [`InterpolationKind::Linear`] uses straight-line
/// interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum InterpolationKind {
    Smooth,
    Linear,
}

impl Default for InterpolationKind {
    fn default() -> Self {
        Self::Smooth
    }
}

/// Options accepted by [`Viewport`]-changing helpers such as
/// `set_viewport`, `zoom_in`, `zoom_to`.
///
/// All fields are optional so that the empty struct (`..Default`) is a
/// valid "no animation" call.
#[derive(Default)]
pub struct ViewportHelperFunctionOptions {
    /// Animation duration in milliseconds. `None` = jump instantly.
    pub duration: Option<f64>,
    /// Easing curve `t -> t'` applied to the duration.
    pub ease: Option<EaseFn>,
    /// Interpolation strategy. Defaults to
    /// [`InterpolationKind::Smooth`].
    pub interpolate: Option<InterpolationKind>,
}

impl std::fmt::Debug for ViewportHelperFunctionOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViewportHelperFunctionOptions")
            .field("duration", &self.duration)
            .field("ease", &self.ease.as_ref().map(|_| "<fn>"))
            .field("interpolate", &self.interpolate)
            .finish()
    }
}

/// Options for setting the center of the flow viewport to a specific
/// position. Extends [`ViewportHelperFunctionOptions`] with an optional
/// override zoom.
#[derive(Default, Debug)]
pub struct SetCenterOptions {
    pub base: ViewportHelperFunctionOptions,
    pub zoom: Option<f64>,
}

/// Options for fitting the viewport to a [`Rect`] of node bounds.
/// Extends [`ViewportHelperFunctionOptions`] with an optional padding
/// override (in pixels around the bounds).
#[derive(Default, Debug)]
pub struct FitBoundsOptions {
    pub base: ViewportHelperFunctionOptions,
    pub padding: Option<f64>,
}

/// Options for `fit_view` / `fit_viewport`.
///
/// Mirrors the TS `FitViewOptionsBase<NodeType>`. The `nodes` field is
/// a list of node ids to focus on (TS allows full nodes too; the Rust
/// port narrows that to ids since pointer equality across boundaries
/// isn't useful here).
#[derive(Default)]
pub struct FitViewOptionsBase {
    pub padding: Option<Padding>,
    pub include_hidden_nodes: bool,
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub duration: Option<f64>,
    pub ease: Option<EaseFn>,
    pub interpolate: Option<InterpolationKind>,
    /// Restrict fit-view to these node ids only.
    pub nodes: Option<Vec<String>>,
}

impl std::fmt::Debug for FitViewOptionsBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FitViewOptionsBase")
            .field("padding", &self.padding)
            .field("include_hidden_nodes", &self.include_hidden_nodes)
            .field("min_zoom", &self.min_zoom)
            .field("max_zoom", &self.max_zoom)
            .field("duration", &self.duration)
            .field("ease", &self.ease.as_ref().map(|_| "<fn>"))
            .field("interpolate", &self.interpolate)
            .field("nodes", &self.nodes)
            .finish()
    }
}

/// Snap-grid pair `(width, height)`, e.g. `(15.0, 15.0)`.
///
/// Mirrors the TS `SnapGrid = [number, number]`.
pub type SnapGrid = (f64, f64);

/// Padding values, with three forms accepted by the xyflow API:
///
/// * a bare number (treated as a zoom-relative factor),
/// * a string ending in `px` (absolute pixels), or
/// * a string ending in `%` (percentage of the viewport).
///
/// Mirrors the TS `PaddingWithUnit = number | `${number}${PaddingUnit}``.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaddingWithUnit {
    /// Bare number — relative factor.
    Number(f64),
    /// Pixels.
    Px(f64),
    /// Percentage of viewport (`50` ≡ `50%`).
    Percent(f64),
}

/// Padding for `getViewportForBounds` / `fitView`.
///
/// Mirrors the TS:
/// `Padding = PaddingWithUnit | { top?, right?, bottom?, left?, x?, y? }`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Padding {
    /// Single padding applied to all sides.
    Single(PaddingWithUnit),
    /// Per-side padding. `x` / `y` act as horizontal / vertical
    /// shorthands when individual sides are not set; sides explicitly
    /// set take precedence.
    Sided {
        top: Option<PaddingWithUnit>,
        right: Option<PaddingWithUnit>,
        bottom: Option<PaddingWithUnit>,
        left: Option<PaddingWithUnit>,
        x: Option<PaddingWithUnit>,
        y: Option<PaddingWithUnit>,
    },
    /// No padding at all (unset). Matches `0` in the TS API.
    #[default]
    Zero,
}

impl Padding {
    /// Convenience constructor for "same padding on every side".
    #[must_use]
    pub const fn uniform(p: PaddingWithUnit) -> Self {
        Padding::Single(p)
    }

    /// Convenience constructor for the JS shorthand `padding: 0.1`
    /// (a relative factor).
    #[must_use]
    pub const fn factor(value: f64) -> Self {
        Padding::Single(PaddingWithUnit::Number(value))
    }
}

/// Selection rectangle used during marquee-style multi-select.
///
/// Mirrors the TS `SelectionRect = Rect & { startX: number; startY: number }`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SelectionRect {
    pub rect: Rect,
    pub start_x: f64,
    pub start_y: f64,
}

/// Position of a fixed "panel" component (Controls, MiniMap, …) over
/// the flow viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum PanelPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    CenterLeft,
    CenterRight,
}

/// React Flow Pro options, kept for parity. Most consumers won't use
/// this in `rgraph-core` directly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProOptions {
    pub account: Option<String>,
    pub hide_attribution: bool,
}

/// Two-state colour-mode class used in CSS root data attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ColorModeClass {
    Light,
    Dark,
}

/// Resolved or auto colour mode. `System` defers to `prefers-color-scheme`
/// at the consumer layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ColorMode {
    Light,
    Dark,
    System,
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::System
    }
}

/// How z-indexing is calculated for nodes and edges.
///
/// * `Auto`   — automatically manage z-indexing for selections and sub-flows.
/// * `Basic`  — only manage z-indexing for selections.
/// * `Manual` — apply no automatic z-indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ZIndexMode {
    #[default]
    Auto,
    Basic,
    Manual,
}

/// Single key code (e.g. `"Space"`) or a list of equivalent codes.
///
/// Mirrors the TS `KeyCode = string | string[]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum KeyCode {
    Single(String),
    Multiple(Vec<String>),
}

impl From<&str> for KeyCode {
    fn from(s: &str) -> Self {
        KeyCode::Single(s.to_string())
    }
}

impl From<String> for KeyCode {
    fn from(s: String) -> Self {
        KeyCode::Single(s)
    }
}

impl From<Vec<String>> for KeyCode {
    fn from(v: Vec<String>) -> Self {
        KeyCode::Multiple(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_identity_default() {
        assert_eq!(Viewport::default(), Viewport::IDENTITY);
        assert_eq!(Viewport::IDENTITY.zoom, 1.0);
    }

    #[test]
    fn padding_constructors() {
        match Padding::factor(0.1) {
            Padding::Single(PaddingWithUnit::Number(n)) => assert!((n - 0.1).abs() < 1e-12),
            _ => panic!("wrong variant"),
        }
        match Padding::uniform(PaddingWithUnit::Px(10.0)) {
            Padding::Single(PaddingWithUnit::Px(n)) => assert!((n - 10.0).abs() < 1e-12),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn keycode_from_conversions() {
        assert_eq!(KeyCode::from("Space"), KeyCode::Single("Space".into()));
        assert_eq!(KeyCode::from("Space".to_string()), KeyCode::Single("Space".into()));
        assert_eq!(
            KeyCode::from(vec!["A".into(), "B".into()]),
            KeyCode::Multiple(vec!["A".into(), "B".into()])
        );
    }

    #[test]
    fn enum_defaults() {
        assert_eq!(PanOnScrollMode::default(), PanOnScrollMode::Free);
        assert_eq!(SelectionMode::default(), SelectionMode::Full);
        assert_eq!(ColorMode::default(), ColorMode::System);
        assert_eq!(ZIndexMode::default(), ZIndexMode::Auto);
        assert_eq!(InterpolationKind::default(), InterpolationKind::Smooth);
    }
}
