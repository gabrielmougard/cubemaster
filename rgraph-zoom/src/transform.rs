//! Affine zoom transform — port of d3-zoom's `transform.js`.
//!
//! A [`Transform`] represents a uniform scale + translation operating
//! on 2D points: `(x, y) -> (x*k + tx, y*k + ty)`. The defaults
//! ([`Transform::IDENTITY`]) leave the input untouched.
//!
//! All methods are pure: each builder (`scale`, `translate`) returns a
//! fresh `Transform` so transforms can be chained without aliasing.
//!
//! # Conventions
//!
//! * `k` is the scale factor (`1.0` = no zoom).
//! * `(x, y)` is the **post-scale** translation — that is, the `(x, y)`
//!   field of the transform stores the translation that has already been
//!   multiplied by `k`. This matches d3 exactly so values round-trip
//!   between the two implementations bit-for-bit.

use core::fmt;

/// Affine zoom transform: `point -> point * k + (x, y)`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Transform {
    /// Scale factor.
    pub k: f64,
    /// Post-scale x translation.
    pub x: f64,
    /// Post-scale y translation.
    pub y: f64,
}

impl Transform {
    /// The identity transform: `k = 1`, `x = y = 0`. Equivalent to d3's
    /// `zoomIdentity`.
    pub const IDENTITY: Self = Transform { k: 1.0, x: 0.0, y: 0.0 };

    /// Build a transform from raw fields.
    #[inline]
    pub const fn new(k: f64, x: f64, y: f64) -> Self { Transform { k, x, y } }

    /// Compose with an additional uniform scale `k`. Mirrors d3's
    /// `transform.scale(k)`. Returns the same transform unchanged when
    /// `k == 1.0`.
    #[inline]
    pub fn scale(self, k: f64) -> Self {
        if k == 1.0 { self } else { Transform { k: self.k * k, x: self.x, y: self.y } }
    }

    /// Compose with an additional translation `(x, y)` in **untransformed**
    /// coordinates (the values get pre-multiplied by `self.k` to match
    /// the d3 convention). Returns the same transform unchanged when both
    /// deltas are zero.
    #[inline]
    pub fn translate(self, x: f64, y: f64) -> Self {
        if x == 0.0 && y == 0.0 {
            self
        } else {
            Transform { k: self.k, x: self.x + self.k * x, y: self.y + self.k * y }
        }
    }

    /// Apply the transform to a point. Returns `point * k + (x, y)`.
    #[inline]
    pub fn apply(self, point: [f64; 2]) -> [f64; 2] {
        [point[0] * self.k + self.x, point[1] * self.k + self.y]
    }

    /// Apply only the x component.
    #[inline]
    pub fn apply_x(self, x: f64) -> f64 { x * self.k + self.x }

    /// Apply only the y component.
    #[inline]
    pub fn apply_y(self, y: f64) -> f64 { y * self.k + self.y }

    /// Inverse mapping: `(location - (x, y)) / k`. Mirrors d3's
    /// `transform.invert(location)`.
    #[inline]
    pub fn invert(self, location: [f64; 2]) -> [f64; 2] {
        [(location[0] - self.x) / self.k, (location[1] - self.y) / self.k]
    }

    /// Inverse of [`apply_x`](Self::apply_x).
    #[inline]
    pub fn invert_x(self, x: f64) -> f64 { (x - self.x) / self.k }

    /// Inverse of [`apply_y`](Self::apply_y).
    #[inline]
    pub fn invert_y(self, y: f64) -> f64 { (y - self.y) / self.k }

    /// Format as an SVG `transform` attribute string. Matches d3's
    /// `transform.toString()` byte-for-byte.
    pub fn to_svg_string(self) -> String {
        format!("translate({},{}) scale({})", self.x, self.y, self.k)
    }
}

impl Default for Transform {
    fn default() -> Self { Self::IDENTITY }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_svg_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_k1_x0_y0() {
        let t = Transform::IDENTITY;
        assert_eq!(t.k, 1.0);
        assert_eq!(t.x, 0.0);
        assert_eq!(t.y, 0.0);
    }

    #[test]
    fn scale_chains_multiplicatively() {
        // d3 fixture: zoomIdentity.scale(2.5).scale(2) == ZoomTransform(5, 0, 0)
        let t = Transform::IDENTITY.scale(2.5).scale(2.0);
        assert_eq!(t, Transform::new(5.0, 0.0, 0.0));
    }

    #[test]
    fn scale_by_one_returns_self_unchanged() {
        // d3 short-circuit: `k === 1 ? this : ...`
        let t = Transform::IDENTITY.translate(2.0, 3.0);
        assert_eq!(t.scale(1.0), t);
    }

    #[test]
    fn translate_pre_multiplies_by_k() {
        // d3 fixture: zoomIdentity.translate(2,3).translate(-4,4) == (1, -2, 7)
        let t = Transform::IDENTITY.translate(2.0, 3.0).translate(-4.0, 4.0);
        assert_eq!(t, Transform::new(1.0, -2.0, 7.0));
        // d3 fixture: zoomIdentity.translate(2,3).scale(2).translate(-4,4) == (2, -6, 11)
        let t2 = Transform::IDENTITY.translate(2.0, 3.0).scale(2.0).translate(-4.0, 4.0);
        assert_eq!(t2, Transform::new(2.0, -6.0, 11.0));
    }

    #[test]
    fn translate_by_zero_zero_returns_self() {
        let t = Transform::IDENTITY.scale(3.0);
        assert_eq!(t.translate(0.0, 0.0), t);
    }

    #[test]
    fn apply_matches_d3_fixture() {
        // d3: zoomIdentity.translate(2,3).scale(2).apply([4,5]) == [10, 13]
        let t = Transform::IDENTITY.translate(2.0, 3.0).scale(2.0);
        assert_eq!(t.apply([4.0, 5.0]), [10.0, 13.0]);
    }

    #[test]
    fn apply_x_and_y_match_d3() {
        let t = Transform::IDENTITY.translate(2.0, 0.0).scale(2.0);
        assert_eq!(t.apply_x(4.0), 10.0);
        let t = Transform::IDENTITY.translate(0.0, 3.0).scale(2.0);
        assert_eq!(t.apply_y(5.0), 13.0);
    }

    #[test]
    fn invert_matches_d3_fixture() {
        // d3: zoomIdentity.translate(2,3).scale(2).invert([4,5]) == [1, 1]
        let t = Transform::IDENTITY.translate(2.0, 3.0).scale(2.0);
        assert_eq!(t.invert([4.0, 5.0]), [1.0, 1.0]);
    }

    #[test]
    fn invert_x_and_y_match_d3() {
        let t = Transform::IDENTITY.translate(2.0, 0.0).scale(2.0);
        assert_eq!(t.invert_x(4.0), 1.0);
        let t = Transform::IDENTITY.translate(0.0, 3.0).scale(2.0);
        assert_eq!(t.invert_y(5.0), 1.0);
    }

    #[test]
    fn apply_then_invert_round_trips() {
        let t = Transform::IDENTITY.translate(7.0, -3.5).scale(0.42);
        for &p in &[[0.0, 0.0], [1.0, 1.0], [-100.0, 200.0], [1e-3, 1e6]] {
            let q = t.invert(t.apply(p));
            assert!((p[0] - q[0]).abs() < 1e-9);
            assert!((p[1] - q[1]).abs() < 1e-9);
        }
    }

    #[test]
    fn to_svg_string_matches_d3() {
        // d3: zoomIdentity.toString() == "translate(0,0) scale(1)"
        assert_eq!(Transform::IDENTITY.to_svg_string(), "translate(0,0) scale(1)");
        // Display impl forwards to to_svg_string.
        assert_eq!(format!("{}", Transform::IDENTITY), "translate(0,0) scale(1)");
    }

    #[test]
    fn default_is_identity() {
        assert_eq!(Transform::default(), Transform::IDENTITY);
    }
}
