//! Hue interpolation — port of d3-interpolate's `hue.js`.
//!
//! Interpolates two hue values (in degrees) along the **shortest** path
//! around the 360° cycle and wraps the result back into `[0, 360)`.

use crate::color_helpers::hue as hue_helper;

/// Hue interpolation. Result is wrapped to `[0, 360)`.
///
/// Equivalent to d3's `interpolateHue(a, b)`. NaN endpoints fall back
/// to the other endpoint; if both are NaN the result is NaN.
pub fn interpolate_hue(a: f64, b: f64) -> impl Fn(f64) -> f64 {
    let inner = hue_helper(a, b);
    move |t: f64| {
        let x = inner(t);
        if x.is_nan() { x } else { x - 360.0 * (x / 360.0).floor() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    }

    #[test]
    fn wraps_to_0_360() {
        // a=350, b=10 (diff 20 short path forward), midpoint should
        // be 360 -> wraps to 0.
        let i = interpolate_hue(350.0, 10.0);
        close(i(0.0), 350.0);
        close(i(0.5), 0.0);
        close(i(1.0), 10.0);
    }

    #[test]
    fn within_180_direct() {
        let i = interpolate_hue(100.0, 200.0);
        close(i(0.5), 150.0);
    }

    #[test]
    fn nan_handling() {
        close(interpolate_hue(f64::NAN, 42.0)(0.5), 42.0);
        close(interpolate_hue(42.0, f64::NAN)(0.5), 42.0);
    }
}
