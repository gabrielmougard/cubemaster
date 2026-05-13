//! Smooth zoom-and-pan interpolation — port of d3-interpolate's
//! `zoom.js`.
//!
//! Produces an interpolator from one viewport `(ux, uy, w)` to another,
//! where `(ux, uy)` is the focus point and `w` is the viewport width
//! (in user units, decoupled from screen pixels). The trajectory is a
//! parametric curve designed by van Wijk & Nuij (2003) that minimises
//! perceived motion sickness when chaining a zoom and a pan.
//!
//! The interpolator reports a *recommended duration* in milliseconds,
//! computed from the path's intrinsic length. d3 uses this in
//! `transition.duration(d3.interpolateZoom(p0, p1).duration)`.

const EPSILON2: f64 = 1e-12;

#[inline]
fn cosh_x(mut x: f64) -> f64 { x = x.exp(); (x + 1.0 / x) / 2.0 }
#[inline]
fn sinh_x(mut x: f64) -> f64 { x = x.exp(); (x - 1.0 / x) / 2.0 }
#[inline]
fn tanh_x(mut x: f64) -> f64 { x = (2.0 * x).exp(); (x - 1.0) / (x + 1.0) }

/// Smooth zoom-and-pan interpolator.
///
/// `at(t)` returns the `[ux, uy, w]` triple at parameter `t ∈ [0, 1]`.
/// `duration` is the recommended animation duration in ms (d3 reports
/// the same value from `interpolator.duration`).
///
/// Pre-allocate one `ZoomInterpolator` per zoom-pan; calling `at` is a
/// pure arithmetic operation with no further allocation.
pub struct ZoomInterpolator {
    /// Recommended animation duration in milliseconds. Mirrors d3's
    /// `interpolator.duration` field. May be 0.0 when start == end.
    pub duration: f64,
    /// Pre-computed parameters used by `at`.
    inner: ZoomShape,
    rho: f64,
    rho2: f64,
}

#[derive(Clone, Copy)]
enum ZoomShape {
    /// Special case: source and destination focus points are
    /// (effectively) coincident; only `w` changes. Pure exponential
    /// zoom-only path.
    Coincident { ux0: f64, uy0: f64, w0: f64, dx: f64, dy: f64, s_total: f64 },
    /// General case: hyperbolic-trigonometric trajectory.
    General {
        ux0: f64, uy0: f64, w0: f64,
        dx: f64, dy: f64, d1: f64,
        r0: f64,
        cosh_r0: f64,
        s_total: f64,
    },
}

impl ZoomInterpolator {
    /// Construct an interpolator with d3's default `rho = sqrt(2)`.
    pub fn new(p0: [f64; 3], p1: [f64; 3]) -> Self {
        Self::with_rho(p0, p1, std::f64::consts::SQRT_2)
    }

    /// Construct with a custom `rho`. Smaller `rho` (down to 0) makes
    /// the zoom more linear; larger `rho` produces more dramatic curves
    /// and longer durations.
    ///
    /// d3 clamps the user input to `>= 1e-3`; we do the same.
    pub fn with_rho(p0: [f64; 3], p1: [f64; 3], rho: f64) -> Self {
        let rho = rho.max(1e-3);
        let rho2 = rho * rho;
        let rho4 = rho2 * rho2;

        let [ux0, uy0, w0] = p0;
        let [ux1, uy1, w1] = p1;
        let dx = ux1 - ux0;
        let dy = uy1 - uy0;
        let d2 = dx * dx + dy * dy;

        let (inner, s_total) = if d2 < EPSILON2 {
            // Source and destination focus coincide.
            let s = (w1 / w0).ln() / rho;
            (
                ZoomShape::Coincident {
                    ux0, uy0, w0, dx, dy, s_total: s,
                },
                s,
            )
        } else {
            let d1 = d2.sqrt();
            let b0 = (w1 * w1 - w0 * w0 + rho4 * d2) / (2.0 * w0 * rho2 * d1);
            let b1 = (w1 * w1 - w0 * w0 - rho4 * d2) / (2.0 * w1 * rho2 * d1);
            let r0 = ((b0 * b0 + 1.0).sqrt() - b0).ln();
            let r1 = ((b1 * b1 + 1.0).sqrt() - b1).ln();
            let s = (r1 - r0) / rho;
            let cosh_r0 = cosh_x(r0);
            (
                ZoomShape::General { ux0, uy0, w0, dx, dy, d1, r0, cosh_r0, s_total: s },
                s,
            )
        };

        // d3's duration: `S * 1000 * rho / Math.SQRT2`.
        let duration = s_total * 1000.0 * rho / std::f64::consts::SQRT_2;

        ZoomInterpolator { duration, inner, rho, rho2 }
    }

    /// Evaluate at `t ∈ [0, 1]`. Returns the `[ux, uy, w]` triple.
    pub fn at(&self, t: f64) -> [f64; 3] {
        match self.inner {
            ZoomShape::Coincident { ux0, uy0, w0, dx, dy, s_total } => {
                [
                    ux0 + t * dx,
                    uy0 + t * dy,
                    w0 * (self.rho * t * s_total).exp(),
                ]
            }
            ZoomShape::General { ux0, uy0, w0, dx, dy, d1, r0, cosh_r0, s_total } => {
                let s = t * s_total;
                let u = w0 / (self.rho2 * d1)
                    * (cosh_r0 * tanh_x(self.rho * s + r0) - sinh_x(r0));
                [
                    ux0 + u * dx,
                    uy0 + u * dy,
                    w0 * cosh_r0 / cosh_x(self.rho * s + r0),
                ]
            }
        }
    }
}

/// Convenience constructor matching d3's `interpolateZoom(a, b)` shape.
///
/// Returns the [`ZoomInterpolator`]; access `.at(t)` and `.duration`.
pub fn interpolate_zoom(p0: [f64; 3], p1: [f64; 3]) -> ZoomInterpolator {
    ZoomInterpolator::new(p0, p1)
}

/// `interpolate_zoom.rho(rho)(p0, p1)` equivalent.
pub fn interpolate_zoom_rho(rho: f64, p0: [f64; 3], p1: [f64; 3]) -> ZoomInterpolator {
    ZoomInterpolator::with_rho(p0, p1, rho)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, eps: f64) {
        assert!((a - b).abs() < eps, "{a} vs {b}, |delta|={}", (a - b).abs());
    }

    fn close_arr(a: [f64; 3], b: [f64; 3], eps: f64) {
        for i in 0..3 {
            close(a[i], b[i], eps);
        }
    }

    #[test]
    fn coincident_points_match_d3_fixture() {
        // d3 test:
        // interpolateZoom([324.687…, 59.435…, 1.882…],
        //                 [324.687…, 59.435…, 7.399…])(0.5)
        //   == [324.687210931…, 59.435016017…, 3.732331318…]
        let z = interpolate_zoom(
            [324.687_210_968_036_14, 59.435_016_024_337_61, 1.882_713_739_956_262_1],
            [324.687_210_894_679_4, 59.435_016_010_627_63, 7.399_052_110_984_391],
        );
        let v = z.at(0.5);
        close_arr(
            v,
            [324.687_210_931_357_75, 59.435_016_017_482_62, 3.732_331_318_626_830_5],
            1e-9,
        );
    }

    #[test]
    fn duration_default_rho() {
        // d3 reports specific durations for these inputs.
        close(interpolate_zoom([0.0, 0.0, 1.0], [0.0, 0.0, 1.1]).duration, 67.0, 1.0);
        close(interpolate_zoom([0.0, 0.0, 1.0], [0.0, 0.0, 2.0]).duration, 490.0, 1.0);
        close(interpolate_zoom([0.0, 0.0, 1.0], [10.0, 0.0, 8.0]).duration, 2872.5, 1.0);
    }

    #[test]
    fn rho_default_is_sqrt2() {
        let z1 = interpolate_zoom([0.0, 0.0, 1.0], [10.0, 10.0, 5.0]);
        let z2 = interpolate_zoom_rho(std::f64::consts::SQRT_2, [0.0, 0.0, 1.0], [10.0, 10.0, 5.0]);
        let v1 = z1.at(0.5);
        let v2 = z2.at(0.5);
        close_arr(v1, v2, 1e-12);
    }

    #[test]
    fn rho_zero_is_almost_linear() {
        let z = interpolate_zoom_rho(0.0, [0.0, 0.0, 1.0], [10.0, 0.0, 8.0]);
        let v = z.at(0.5);
        close_arr(v, [1.111, 0.0, 8.0_f64.sqrt()], 1e-3);
        let dur = z.duration.round();
        close(dur, 1470.0, 1.0);
    }

    #[test]
    fn rho_two_is_more_curved() {
        let z = interpolate_zoom_rho(2.0, [0.0, 0.0, 1.0], [10.0, 0.0, 8.0]);
        let v = z.at(0.5);
        close_arr(v, [1.111, 0.0, 12.885], 1e-3);
        close(z.duration.round(), 3775.0, 1.0);
    }

    #[test]
    fn endpoints_exact() {
        let z = interpolate_zoom([0.0, 0.0, 1.0], [10.0, 0.0, 8.0]);
        // t=0 should reproduce p0 (within fp roundoff).
        close_arr(z.at(0.0), [0.0, 0.0, 1.0], 1e-12);
        // t=1 should reproduce p1.
        close_arr(z.at(1.0), [10.0, 0.0, 8.0], 1e-9);
    }
}
