//! Port of `xyflow-core/src/types/utils.ts` — basic 2D geometry types.
//!
//! Status: implemented (phase 1).
//!
//! These types are deliberately small, `Copy` where possible, and serde-
//! aware behind the `serde` feature. They mirror the TS shapes 1:1; the
//! one rename is `Box` → [`Box2d`] because `std::boxed::Box` is in
//! prelude.

#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// While [`crate::types::viewport::PanelPosition`] can be used to place
/// a component in the corners of a container, this enum is less precise
/// and used primarily in relation to edges and handles.
///
/// Mirrors the TS `Position` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Position {
    Left,
    Top,
    Right,
    Bottom,
}

impl Position {
    /// Returns the position on the opposite side of a handle/edge.
    ///
    /// Equivalent of the TS `oppositePosition` lookup map.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Position::Left => Position::Right,
            Position::Right => Position::Left,
            Position::Top => Position::Bottom,
            Position::Bottom => Position::Top,
        }
    }
}

/// Free-standing constant kept for parity with the JS `oppositePosition`
/// export, which downstream code occasionally indexes into directly.
///
/// Prefer [`Position::opposite`] in new code.
#[must_use]
pub const fn opposite_position(p: Position) -> Position {
    p.opposite()
}

/// All 2D positions are stored in an object with `x` and `y`
/// coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct XYPosition {
    pub x: f64,
    pub y: f64,
}

impl XYPosition {
    /// `XYPosition { x: 0.0, y: 0.0 }`.
    pub const ZERO: XYPosition = XYPosition { x: 0.0, y: 0.0 };

    #[inline]
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        XYPosition { x, y }
    }
}

impl From<(f64, f64)> for XYPosition {
    #[inline]
    fn from((x, y): (f64, f64)) -> Self {
        XYPosition { x, y }
    }
}

impl From<XYPosition> for (f64, f64) {
    #[inline]
    fn from(p: XYPosition) -> Self {
        (p.x, p.y)
    }
}

/// 3D position used for nodes that participate in z-stacking.
///
/// Mirrors the TS `XYZPosition = XYPosition & { z: number }`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct XYZPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl XYZPosition {
    pub const ZERO: XYZPosition = XYZPosition {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    #[inline]
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        XYZPosition { x, y, z }
    }
}

impl From<XYZPosition> for XYPosition {
    #[inline]
    fn from(p: XYZPosition) -> Self {
        XYPosition { x: p.x, y: p.y }
    }
}

/// Width / height pair.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Dimensions {
    pub width: f64,
    pub height: f64,
}

impl Dimensions {
    pub const ZERO: Dimensions = Dimensions {
        width: 0.0,
        height: 0.0,
    };

    #[inline]
    #[must_use]
    pub const fn new(width: f64, height: f64) -> Self {
        Dimensions { width, height }
    }
}

/// `Rect` is the rectangle representation used everywhere a node or
/// region's bounds are needed: an [`XYPosition`] origin plus
/// [`Dimensions`].
///
/// Mirrors the TS `Rect = Dimensions & XYPosition`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    #[inline]
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect { x, y, width, height }
    }

    #[inline]
    #[must_use]
    pub const fn position(&self) -> XYPosition {
        XYPosition { x: self.x, y: self.y }
    }

    #[inline]
    #[must_use]
    pub const fn dimensions(&self) -> Dimensions {
        Dimensions {
            width: self.width,
            height: self.height,
        }
    }
}

/// Box representation of a region: top-left + bottom-right corners.
///
/// Renamed from the TS `Box` to avoid clashing with `std::boxed::Box`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Box2d {
    pub x: f64,
    pub y: f64,
    pub x2: f64,
    pub y2: f64,
}

impl Box2d {
    pub const ZERO: Box2d = Box2d {
        x: 0.0,
        y: 0.0,
        x2: 0.0,
        y2: 0.0,
    };

    #[inline]
    #[must_use]
    pub const fn new(x: f64, y: f64, x2: f64, y2: f64) -> Self {
        Box2d { x, y, x2, y2 }
    }
}

/// Affine viewport transform: `(translate_x, translate_y, scale)`.
///
/// Matches the TS `Transform = [number, number, number]`. Stored as a
/// tuple struct so call sites can destructure with
/// `let Transform(tx, ty, scale) = …;`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Transform(pub f64, pub f64, pub f64);

impl Transform {
    /// Identity transform: no translation, scale = 1.
    pub const IDENTITY: Transform = Transform(0.0, 0.0, 1.0);

    #[inline]
    #[must_use]
    pub const fn new(tx: f64, ty: f64, scale: f64) -> Self {
        Transform(tx, ty, scale)
    }

    #[inline]
    #[must_use]
    pub const fn tx(&self) -> f64 {
        self.0
    }

    #[inline]
    #[must_use]
    pub const fn ty(&self) -> f64 {
        self.1
    }

    #[inline]
    #[must_use]
    pub const fn scale(&self) -> f64 {
        self.2
    }
}

impl From<(f64, f64, f64)> for Transform {
    #[inline]
    fn from(t: (f64, f64, f64)) -> Self {
        Transform(t.0, t.1, t.2)
    }
}

impl From<Transform> for (f64, f64, f64) {
    #[inline]
    fn from(t: Transform) -> Self {
        (t.0, t.1, t.2)
    }
}

impl From<[f64; 3]> for Transform {
    #[inline]
    fn from([tx, ty, k]: [f64; 3]) -> Self {
        Transform(tx, ty, k)
    }
}

/// A coordinate extent represents two points in a coordinate system:
/// one in the top-left corner and one in the bottom-right corner. It is
/// used to represent the bounds of nodes in the flow or the bounds of
/// the viewport.
///
/// Mirrors the TS `CoordinateExtent = [[number, number], [number, number]]`.
///
/// Props that expect a `CoordinateExtent` usually default to
/// `[[-∞, -∞], [+∞, +∞]]`.
pub type CoordinateExtent = [[f64; 2]; 2];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opposite_position_round_trips() {
        for p in [Position::Left, Position::Right, Position::Top, Position::Bottom] {
            assert_eq!(p.opposite().opposite(), p);
            assert_eq!(opposite_position(p), p.opposite());
        }
        assert_eq!(Position::Left.opposite(), Position::Right);
        assert_eq!(Position::Top.opposite(), Position::Bottom);
    }

    #[test]
    fn xy_position_conversions() {
        let p: XYPosition = (3.0, 4.0).into();
        assert_eq!(p, XYPosition::new(3.0, 4.0));
        let t: (f64, f64) = p.into();
        assert_eq!(t, (3.0, 4.0));
    }

    #[test]
    fn xyz_position_drops_z_into_xy() {
        let p3 = XYZPosition::new(1.0, 2.0, 3.0);
        let p2: XYPosition = p3.into();
        assert_eq!(p2, XYPosition::new(1.0, 2.0));
    }

    #[test]
    fn rect_destructure_helpers() {
        let r = Rect::new(1.0, 2.0, 10.0, 20.0);
        assert_eq!(r.position(), XYPosition::new(1.0, 2.0));
        assert_eq!(r.dimensions(), Dimensions::new(10.0, 20.0));
    }

    #[test]
    fn transform_identity_and_accessors() {
        let id = Transform::IDENTITY;
        assert_eq!(id.tx(), 0.0);
        assert_eq!(id.ty(), 0.0);
        assert_eq!(id.scale(), 1.0);
        let t: Transform = (5.0, -7.0, 2.0).into();
        let arr: (f64, f64, f64) = t.into();
        assert_eq!(arr, (5.0, -7.0, 2.0));
        let from_arr: Transform = [1.0, 2.0, 3.0].into();
        assert_eq!(from_arr, Transform(1.0, 2.0, 3.0));
    }
}
