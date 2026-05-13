//! RGB-space color interpolators — port of d3-interpolate's `rgb.js`.
//!
//! Linear (default) and gamma-corrected RGB interpolators, plus
//! [`interpolate_rgb_basis`] / [`interpolate_rgb_basis_closed`] for
//! smooth multi-stop color ramps via cubic B-spline.

use rgraph_color::{ColorSpace, Rgb, parse};

use crate::basis::{interpolate_basis, interpolate_basis_closed};
use crate::color_helpers::{gamma, nogamma};

/// Boxed Rgb-string interpolator returned by every public function in
/// this module. Aliased to keep deeply-nested factory signatures readable.
pub type RgbInterp = Box<dyn Fn(f64) -> String>;

/// Boxed factory mapping `(a, b)` Rgbs to an [`RgbInterp`].
pub type RgbFactory = Box<dyn Fn(Rgb, Rgb) -> RgbInterp>;

/// Same factory, but accepting CSS color strings.
pub type RgbStrFactory = Box<dyn Fn(&str, &str) -> RgbInterp>;

/// Parse a string into a [`Color`] using rgraph-color, or fall back to
/// the d3 "null endpoint" semantic: a fully NaN RGB which causes
/// `nogamma` to defer to the other endpoint.
fn parse_or_nan(s: &str) -> Rgb {
    match parse(s) {
        Some(c) => c.rgb(),
        None => Rgb::empty(),
    }
}

/// Linear RGB interpolation between two colors specified as strings.
///
/// Equivalent to d3's `interpolateRgb(a, b)`. Returns a closure that
/// produces an `rgb(...)` / `rgba(...)` formatted string.
///
/// d3's NaN semantic is preserved: if one endpoint's channel is NaN, the
/// other endpoint's value is used as a constant for that channel.
pub fn interpolate_rgb_str(a: &str, b: &str) -> RgbInterp {
    let ra = parse_or_nan(a);
    let rb = parse_or_nan(b);
    interpolate_rgb(ra, rb)
}

/// Linear RGB interpolation between two pre-parsed [`Rgb`] colors.
pub fn interpolate_rgb(a: Rgb, b: Rgb) -> RgbInterp {
    let r = nogamma(a.r, b.r);
    let g = nogamma(a.g, b.g);
    let bch = nogamma(a.b, b.b);
    let opacity = nogamma(a.opacity, b.opacity);
    Box::new(move |t| {
        let mixed = Rgb::new(r(t), g(t), bch(t), opacity(t));
        mixed.format_rgb()
    })
}

/// Gamma-corrected RGB interpolation factory.
///
/// `interpolate_rgb_gamma(y)` returns a function `(a, b) -> Fn(f64) -> String`
/// that interpolates each channel under the chosen gamma. `y == 1.0` is
/// equivalent to [`interpolate_rgb`]; `y > 1` exaggerates the brighter
/// midtones, `y < 1` darkens them.
///
/// Mirrors d3's `interpolateRgb.gamma(y)`.
pub fn interpolate_rgb_gamma(y: f64) -> RgbFactory {
    let mk_channel = gamma(y);
    let mk_channel: std::rc::Rc<crate::color_helpers::ChannelFactory> =
        std::rc::Rc::from(mk_channel);
    Box::new(move |a, b| {
        let r = mk_channel(a.r, b.r);
        let g = mk_channel(a.g, b.g);
        let bch = mk_channel(a.b, b.b);
        // d3 always uses linear interpolation for opacity, regardless of
        // gamma — matches the rgb-test "uses linear interpolation for
        // opacity" case.
        let opacity = nogamma(a.opacity, b.opacity);
        Box::new(move |t: f64| {
            let mixed = Rgb::new(r(t), g(t), bch(t), opacity(t));
            mixed.format_rgb()
        })
    })
}

/// String-input variant of [`interpolate_rgb_gamma`].
pub fn interpolate_rgb_gamma_str(y: f64) -> RgbStrFactory {
    let mk = interpolate_rgb_gamma(y);
    let mk: std::rc::Rc<RgbFactory> = std::rc::Rc::from(mk);
    Box::new(move |a, b| mk(parse_or_nan(a), parse_or_nan(b)))
}

// ---------------------------------------------------------------------------
// RGB basis splines
// ---------------------------------------------------------------------------

/// Cubic B-spline color ramp through `colors`, in RGB space.
///
/// Equivalent to d3's `interpolateRgbBasis`. Each channel is splined
/// independently. Useful for multi-stop diverging colormaps.
///
/// `colors` may be CSS color strings or hex codes.
pub fn interpolate_rgb_basis(colors: &[&str]) -> RgbInterp {
    rgb_spline_strs(colors, false)
}

/// Closed cubic B-spline color ramp; first and last colors are treated
/// as adjacent (the curve loops). Equivalent to d3's
/// `interpolateRgbBasisClosed`.
pub fn interpolate_rgb_basis_closed(colors: &[&str]) -> RgbInterp {
    rgb_spline_strs(colors, true)
}

fn rgb_spline_strs(colors: &[&str], closed: bool) -> RgbInterp {
    let parsed: Vec<Rgb> = colors.iter().map(|s| parse_or_nan(s)).collect();
    rgb_spline(&parsed, closed)
}

/// Pre-parsed [`Rgb`] basis variant.
pub fn rgb_spline(colors: &[Rgb], closed: bool) -> RgbInterp {
    // d3 substitutes 0 for NaN channels in basis splines (see rgb.js:
    // `r[i] = color.r || 0;`). Faithful port.
    let r: Vec<f64> = colors.iter().map(|c| zero_if_nan(c.r)).collect();
    let g: Vec<f64> = colors.iter().map(|c| zero_if_nan(c.g)).collect();
    let b: Vec<f64> = colors.iter().map(|c| zero_if_nan(c.b)).collect();
    type ChanSplineFn = fn(Vec<f64>) -> Box<dyn Fn(f64) -> f64>;
    let mk: ChanSplineFn = if closed {
        |v| Box::new(interpolate_basis_closed(v))
    } else {
        |v| Box::new(interpolate_basis(v))
    };
    let r = mk(r);
    let g = mk(g);
    let b = mk(b);
    Box::new(move |t| {
        // d3 sets opacity = 1 when constructing from a basis spline.
        Rgb::new(r(t), g(t), b(t), 1.0).format_rgb()
    })
}

#[inline]
fn zero_if_nan(x: f64) -> f64 { if x.is_nan() { 0.0 } else { x } }

// ---------------------------------------------------------------------------
// Convenience: typed-color helpers (avoids parsing for callers that
// already have an Rgb).
// ---------------------------------------------------------------------------

/// Build a linear RGB interpolator from any pair of `ColorSpace` values.
pub fn interpolate_rgb_typed<A: ColorSpace, B: ColorSpace>(a: A, b: B) -> RgbInterp {
    interpolate_rgb(a.rgb(), b.rgb())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_str_endpoints_match_d3() {
        // d3-test: interpolateRgb("steelblue", "brown")(0) == rgb("steelblue") + ""
        let i = interpolate_rgb_str("steelblue", "brown");
        let from = rgraph_color::parse("steelblue").unwrap().rgb().format_rgb();
        let to = rgraph_color::parse("brown").unwrap().rgb().format_rgb();
        assert_eq!(i(0.0), from);
        assert_eq!(i(1.0), to);
    }

    #[test]
    fn rgb_str_midpoint() {
        // d3-test: interpolateRgb("steelblue", "#f00")(0.2) == "rgb(107, 104, 144)"
        let i = interpolate_rgb_str("steelblue", "#f00");
        assert_eq!(i(0.2), "rgb(107, 104, 144)");
    }

    #[test]
    fn rgba_midpoint() {
        // d3-test: interpolateRgb("rgba(70,130,180,1)", "rgba(255,0,0,0.2)")(0.2)
        //          == "rgba(107, 104, 144, 0.84)"
        let i = interpolate_rgb_str("rgba(70, 130, 180, 1)", "rgba(255, 0, 0, 0.2)");
        assert_eq!(i(0.2), "rgba(107, 104, 144, 0.84)");
    }

    #[test]
    fn nan_channel_in_a_uses_b() {
        // d3-test: interpolateRgb(rgb(NaN, 20, 40), rgb(60, 80, 100))(0.5) == rgb(60, 50, 70)
        let a = Rgb::new(f64::NAN, 20.0, 40.0, 1.0);
        let b = Rgb::new(60.0, 80.0, 100.0, 1.0);
        let i = interpolate_rgb(a, b);
        assert_eq!(i(0.5), "rgb(60, 50, 70)");
    }

    #[test]
    fn nan_channel_in_b_uses_a() {
        let a = Rgb::new(60.0, 80.0, 100.0, 1.0);
        let b = Rgb::new(f64::NAN, 20.0, 40.0, 1.0);
        let i = interpolate_rgb(a, b);
        assert_eq!(i(0.5), "rgb(60, 50, 70)");
    }

    #[test]
    fn gamma_one_equals_default() {
        let mk = interpolate_rgb_gamma_str(1.0);
        let a = "purple";
        let b = "orange";
        let i_g1 = mk(a, b);
        let i_def = interpolate_rgb_str(a, b);
        for t in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0] {
            assert_eq!(i_g1(t), i_def(t), "t={t}");
        }
    }

    #[test]
    fn gamma_three_at_02_matches_d3() {
        // d3-test: interpolateRgb.gamma(3)("steelblue", "#f00")(0.2) == "rgb(153, 121, 167)"
        let mk = interpolate_rgb_gamma_str(3.0);
        let i = mk("steelblue", "#f00");
        assert_eq!(i(0.2), "rgb(153, 121, 167)");
    }

    #[test]
    fn gamma_uses_linear_opacity() {
        // d3-test: interpolateRgb.gamma(3)("transparent", "#f00")(0.2)
        //          == "rgba(255, 0, 0, 0.2)"
        let mk = interpolate_rgb_gamma_str(3.0);
        let i = mk("transparent", "#f00");
        assert_eq!(i(0.2), "rgba(255, 0, 0, 0.2)");
    }

    #[test]
    fn rgb_basis_smoke() {
        let i = interpolate_rgb_basis(&["#000", "#f00", "#ff0", "#fff"]);
        // Endpoints aren't exact for B-spline but should be close.
        let s0 = i(0.0);
        let s1 = i(1.0);
        // Just verify it parses to RGB strings.
        assert!(s0.starts_with("rgb"));
        assert!(s1.starts_with("rgb"));
    }

    #[test]
    fn rgb_basis_closed_loops() {
        let i = interpolate_rgb_basis_closed(&["#000", "#f00", "#ff0"]);
        assert_eq!(i(0.0), i(1.0));
    }
}
