//! B-spline interpolation — port of d3-interpolate's `basis.js` and
//! `basisClosed.js`.
//!
//! These produce a smooth interpolator across a sequence of `n` numeric
//! control points using a uniform cubic B-spline. Every interior segment
//! is locally controlled by 4 consecutive points; the open variant
//! ([`interpolate_basis`]) extends the endpoints linearly, while the
//! closed variant ([`interpolate_basis_closed`]) wraps around.

/// One step of cubic B-spline evaluation: given local parameter `t1` and
/// the four control values around the current segment, return the
/// blended position. Mirrors d3's `basis(t1, v0, v1, v2, v3)`.
///
/// Public so callers can reuse it as a building block (e.g. RGB basis
/// splines do exactly that).
#[inline]
pub fn basis_step(t1: f64, v0: f64, v1: f64, v2: f64, v3: f64) -> f64 {
    let t2 = t1 * t1;
    let t3 = t2 * t1;
    ((1.0 - 3.0 * t1 + 3.0 * t2 - t3) * v0
        + (4.0 - 6.0 * t2 + 3.0 * t3) * v1
        + (1.0 + 3.0 * t1 + 3.0 * t2 - 3.0 * t3) * v2
        + t3 * v3)
        / 6.0
}

/// Open B-spline interpolator across the given control values.
///
/// Equivalent to d3's default export from `basis.js`. The endpoints are
/// extended via the rule `v0 = 2*v1 - v2`, which makes the curve pass
/// approximately through the first and last control points.
///
/// # Panics
///
/// Panics if `values` has fewer than 2 entries.
pub fn interpolate_basis(values: Vec<f64>) -> impl Fn(f64) -> f64 {
    assert!(values.len() >= 2, "interpolate_basis: need at least 2 values");
    let n = values.len() - 1;
    move |mut t: f64| {
        // t is clamped to [0, 1]; for t out of range d3 picks the endpoint.
        let i = if t <= 0.0 {
            t = 0.0;
            0
        } else if t >= 1.0 {
            t = 1.0;
            n - 1
        } else {
            (t * n as f64).floor() as usize
        };
        let v1 = values[i];
        let v2 = values[i + 1];
        let v0 = if i > 0 { values[i - 1] } else { 2.0 * v1 - v2 };
        let v3 = if i + 1 < n { values[i + 2] } else { 2.0 * v2 - v1 };
        basis_step((t - i as f64 / n as f64) * n as f64, v0, v1, v2, v3)
    }
}

/// Closed B-spline interpolator. The control values wrap around so the
/// curve loops smoothly. Equivalent to d3's `basisClosed.js`.
///
/// # Panics
///
/// Panics if `values` is empty.
pub fn interpolate_basis_closed(values: Vec<f64>) -> impl Fn(f64) -> f64 {
    assert!(!values.is_empty(), "interpolate_basis_closed: need at least 1 value");
    let n = values.len();
    move |t: f64| {
        // t mod 1, in [0, 1)
        let mut t = t % 1.0;
        if t < 0.0 { t += 1.0; }
        let scaled = t * n as f64;
        let i = scaled.floor() as usize;
        let v0 = values[(i + n - 1) % n];
        let v1 = values[i % n];
        let v2 = values[(i + 1) % n];
        let v3 = values[(i + 2) % n];
        basis_step(scaled - i as f64, v0, v1, v2, v3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    }

    #[test]
    fn basis_step_matches_polynomial() {
        // Hand-verified for v0=0, v1=1, v2=2, v3=3, t1=0.5
        // ((1 - 1.5 + 0.75 - 0.125)*0 + (4 - 1.5 + 0.375)*1
        //  + (1 + 1.5 + 0.75 - 0.375)*2 + 0.125*3) / 6
        // = (0 + 2.875 + 5.75 + 0.375) / 6 = 9 / 6 = 1.5
        close(basis_step(0.5, 0.0, 1.0, 2.0, 3.0), 1.5);
    }

    #[test]
    fn basis_open_endpoints() {
        let i = interpolate_basis(vec![0.0, 10.0]);
        // n=1, so t=0 -> i=0; t=1 -> clamped to i=n-1=0.
        // v0 = 2*v1 - v2 = -10, v3 = 2*v2 - v1 = 20
        // at t=0: basis_step(0, -10, 0, 10, 20) = ?
        //   ((1)*-10 + 4*0 + 1*10 + 0*20)/6 = 0 / 6 = 0
        close(i(0.0), 0.0);
        // at t=1: basis_step(1, -10, 0, 10, 20)
        //   ((1-3+3-1)*-10 + (4-6+3)*0 + (1+3+3-3)*10 + 1*20)/6
        //   = (0 + 0 + 40 + 20)/6 = 10
        close(i(1.0), 10.0);
    }

    #[test]
    fn basis_three_points_smooth() {
        let i = interpolate_basis(vec![0.0, 5.0, 10.0]);
        close(i(0.0), 0.0);
        close(i(1.0), 10.0);
        // Midpoint should be near 5 due to symmetric control points.
        let mid = i(0.5);
        assert!((mid - 5.0).abs() < 1e-9);
    }

    #[test]
    fn basis_closed_wraps() {
        let i = interpolate_basis_closed(vec![0.0, 1.0, 2.0, 3.0]);
        // Closed curve: i(0) == i(1) (mod 1), i(0.5) somewhere mid.
        close(i(0.0), i(1.0));
        // i(-0.5) should equal i(0.5).
        close(i(-0.5), i(0.5));
    }

    #[test]
    #[should_panic]
    fn basis_open_one_value_panics() {
        let _ = interpolate_basis(vec![0.0]);
    }

    #[test]
    #[should_panic]
    fn basis_closed_empty_panics() {
        let _ = interpolate_basis_closed(vec![]);
    }
}
