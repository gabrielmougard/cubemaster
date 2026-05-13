//! Cubehelix color-space interpolation — port of d3-interpolate's
//! `cubehelix.js`.
//!
//! Cubehelix is a one-parameter family of color spirals through RGB
//! cube, designed by Dave Green for grayscale-friendly print colormaps.
//! d3 supports two variants:
//!
//! * default `interpolateCubehelix` — shortest hue path,
//! * `interpolateCubehelixLong` — longer hue path.
//!
//! Both expose a `.gamma(y)` factory to bias the lightness curve.

use rgraph_color::{Cubehelix, cubehelix_from, parse};

use crate::StringInterp;
use crate::color_helpers::{hue, nogamma};

/// Boxed factory for cubehelix interpolators on string endpoints.
pub type CubehelixStrFactory = Box<dyn Fn(&str, &str) -> StringInterp>;

fn parse_cubehelix(s: &str) -> Cubehelix {
    match parse(s) {
        Some(c) => cubehelix_from(&c),
        None => Cubehelix::empty(),
    }
}

/// Cubehelix interpolation taking the **shortest** hue path.
pub fn interpolate_cubehelix_str(a: &str, b: &str) -> StringInterp {
    interpolate_cubehelix_gamma_typed(parse_cubehelix(a), parse_cubehelix(b), 1.0, false)
}

/// Cubehelix interpolation between pre-parsed [`Cubehelix`] values.
pub fn interpolate_cubehelix(a: Cubehelix, b: Cubehelix) -> StringInterp {
    interpolate_cubehelix_gamma_typed(a, b, 1.0, false)
}

/// Cubehelix interpolation taking the **long** hue path.
pub fn interpolate_cubehelix_long_str(a: &str, b: &str) -> StringInterp {
    interpolate_cubehelix_gamma_typed(parse_cubehelix(a), parse_cubehelix(b), 1.0, true)
}

/// "Long" cubehelix between pre-parsed colors.
pub fn interpolate_cubehelix_long(a: Cubehelix, b: Cubehelix) -> StringInterp {
    interpolate_cubehelix_gamma_typed(a, b, 1.0, true)
}

/// Gamma-corrected cubehelix interpolator factory (shortest hue path).
///
/// Equivalent to d3's `interpolateCubehelix.gamma(y)`. Gamma applies to
/// the lightness channel only (matching d3).
pub fn interpolate_cubehelix_gamma_str(y: f64) -> CubehelixStrFactory {
    Box::new(move |a, b| {
        interpolate_cubehelix_gamma_typed(parse_cubehelix(a), parse_cubehelix(b), y, false)
    })
}

/// Gamma-corrected cubehelix between pre-parsed colors (shortest hue path).
pub fn interpolate_cubehelix_gamma(
    a: Cubehelix,
    b: Cubehelix,
    y: f64,
) -> StringInterp {
    interpolate_cubehelix_gamma_typed(a, b, y, false)
}

/// Long-path gamma variant.
pub fn interpolate_cubehelix_long_gamma(
    a: Cubehelix,
    b: Cubehelix,
    y: f64,
) -> StringInterp {
    interpolate_cubehelix_gamma_typed(a, b, y, true)
}

fn interpolate_cubehelix_gamma_typed(
    a: Cubehelix,
    b: Cubehelix,
    y: f64,
    long: bool,
) -> StringInterp {
    let h = if long { nogamma(a.h, b.h) } else { hue(a.h, b.h) };
    let s = nogamma(a.s, b.s);
    let l = nogamma(a.l, b.l);
    let opacity = nogamma(a.opacity, b.opacity);
    Box::new(move |t| {
        // d3 applies gamma to lightness's t exponent: l(pow(t, y)).
        let lt = if y == 1.0 { t } else { t.powf(y) };
        let mixed = Cubehelix::new(h(t), s(t), l(lt), opacity(t));
        format!("{mixed}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_match_input() {
        let i = interpolate_cubehelix_str("steelblue", "brown");
        let s0 = format!("{}", parse_cubehelix("steelblue"));
        let s1 = format!("{}", parse_cubehelix("brown"));
        assert_eq!(i(0.0), s0);
        assert_eq!(i(1.0), s1);
    }

    #[test]
    fn long_and_short_differ_in_general() {
        // For short-vs-long to actually diverge, the unsigned hue
        // distance between endpoints must exceed 180°. In cubehelix:
        //   red    h≈351.81
        //   green  h≈109.96
        // -> raw diff ≈ -241.85, normalized to short path of ≈ +118.15.
        //    short midpoint hue ≈ 410.89° ≡ 50.89° (yellow-ish);
        //    long midpoint hue ≈ 230.89° (blue-ish). Quite different.
        let i_short = interpolate_cubehelix_str("red", "green");
        let i_long = interpolate_cubehelix_long_str("red", "green");
        assert_ne!(i_short(0.5), i_long(0.5));
    }

    #[test]
    fn gamma_one_equivalent_to_default() {
        let mk = interpolate_cubehelix_gamma_str(1.0);
        let i_g1 = mk("steelblue", "brown");
        let i_def = interpolate_cubehelix_str("steelblue", "brown");
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(i_g1(t), i_def(t));
        }
    }

    #[test]
    fn gamma_curves_lightness() {
        // With gamma = 3, lightness reaches its midpoint at sqrt-cube(0.5).
        // Just verify the result differs from gamma=1.
        let mk = interpolate_cubehelix_gamma_str(3.0);
        let i = mk("black", "white");
        let mid = i(0.5);
        // Should be a valid rgb string.
        assert!(mid.starts_with("rgb"));
        // And different from the linear midpoint.
        let i_lin = interpolate_cubehelix_str("black", "white");
        assert_ne!(mid, i_lin(0.5));
    }
}
