//! Discrete and step-based interpolators.
//!
//! * [`interpolate_discrete`] — port of d3's `discrete.js`. Maps `t` to
//!   one of `n` evenly-spaced ranges.
//! * [`quantize`] — port of d3's `quantize.js`. Samples an interpolator
//!   at `n` evenly-spaced points.
//! * [`piecewise`] — port of d3's `piecewise.js`. Stitches together a
//!   sequence of interpolators into a single one.

/// Returns a step function over `range` that maps `t ∈ [0, 1]` to the
/// `floor(t * n)`-th element (clamped to `[0, n-1]`).
///
/// `t < 0` returns `range[0]`; `t >= 1` returns `range[n-1]`.
///
/// Mirrors d3's `interpolateDiscrete(range)`. Returns `None` if `t` is
/// `NaN` (matching d3's behavior of returning `undefined`).
///
/// # Panics
///
/// Panics if `range` is empty.
pub fn interpolate_discrete<T: Clone + 'static>(
    range: Vec<T>,
) -> impl Fn(f64) -> Option<T> {
    assert!(!range.is_empty(), "interpolate_discrete: range must be non-empty");
    let n = range.len();
    move |t: f64| {
        if t.is_nan() { return None; }
        let idx = (t * n as f64).floor();
        let idx = if idx < 0.0 { 0 }
                  else if idx >= n as f64 { n - 1 }
                  else { idx as usize };
        Some(range[idx].clone())
    }
}

/// Sample `interpolator` at `n` evenly-spaced points across `[0, 1]`.
///
/// Equivalent to d3's `quantize(interpolator, n)`. Always emits exactly
/// `n` samples; the first is `interpolator(0)` and the last is
/// `interpolator(1)` (mirroring d3, where the divisor is `n - 1`).
///
/// # Panics
///
/// Panics if `n < 2` — d3 silently produces a `NaN` for `n=1` (because of
/// `0 / 0`); that's almost never what callers want, so we make it a hard
/// error.
pub fn quantize<T, F: FnMut(f64) -> T>(mut interpolator: F, n: usize) -> Vec<T> {
    assert!(n >= 2, "quantize: n must be >= 2");
    let mut out = Vec::with_capacity(n);
    let denom = (n - 1) as f64;
    for i in 0..n {
        out.push(interpolator(i as f64 / denom));
    }
    out
}

/// Stitch together a sequence of interpolators (or values, with a
/// user-supplied interpolator factory) into a single interpolator
/// covering `[0, 1]`.
///
/// `make` is called once per consecutive `(values[i], values[i+1])` pair
/// at construction time and returns a per-segment interpolator. At
/// runtime, `t ∈ [i/N, (i+1)/N]` is forwarded to segment `i` as a
/// rescaled `t' ∈ [0, 1]` (`t' = t * N - i`).
///
/// Equivalent to d3's `piecewise(interpolate, values)`.
///
/// # Panics
///
/// Panics if `values.len() < 2`.
pub fn piecewise<T, F>(values: &[T], mut make: F) -> impl Fn(f64) -> T + use<T, F>
where
    T: Clone + 'static,
    F: FnMut(&T, &T) -> Box<dyn Fn(f64) -> T>,
{
    assert!(values.len() >= 2, "piecewise: need at least 2 values");
    let n = values.len() - 1;
    let mut interps: Vec<Box<dyn Fn(f64) -> T>> = Vec::with_capacity(n);
    for i in 0..n {
        interps.push(make(&values[i], &values[i + 1]));
    }
    let n_f = n as f64;
    move |t: f64| {
        let scaled = t * n_f;
        let i_f = scaled.floor();
        // Clamp to [0, n-1] like d3.
        let i = if i_f < 0.0 { 0 }
                else if i_f as usize >= n { n - 1 }
                else { i_f as usize };
        interps[i](scaled - i as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discrete_strings() {
        let i = interpolate_discrete(
            vec!["a", "b", "c", "d", "e"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>(),
        );
        // From d3 test: t=-1 -> "a", t=0 -> "a", t=0.19 -> "a",
        // t=0.21 -> "b", t=1 -> "e".
        assert_eq!(i(-1.0).unwrap(), "a");
        assert_eq!(i(0.0).unwrap(), "a");
        assert_eq!(i(0.19).unwrap(), "a");
        assert_eq!(i(0.21).unwrap(), "b");
        assert_eq!(i(1.0).unwrap(), "e");
    }

    #[test]
    fn discrete_two_values_acts_like_round() {
        let i = interpolate_discrete(vec![0u32, 1]);
        // From d3 test
        assert_eq!(i(-1.0).unwrap(), 0);
        assert_eq!(i(0.0).unwrap(), 0);
        assert_eq!(i(0.49).unwrap(), 0);
        assert_eq!(i(0.51).unwrap(), 1);
        assert_eq!(i(1.0).unwrap(), 1);
        assert_eq!(i(2.0).unwrap(), 1);
    }

    #[test]
    fn discrete_nan_returns_none() {
        let i = interpolate_discrete(vec![0u32, 1]);
        assert_eq!(i(f64::NAN), None);
    }

    #[test]
    #[should_panic(expected = "non-empty")]
    fn discrete_empty_panics() {
        let _ = interpolate_discrete::<u32>(vec![]);
    }

    #[test]
    fn quantize_uniform_samples() {
        // From d3 test: quantize(interpolateNumber(0,1), 5) == [0, 0.25, 0.5, 0.75, 1]
        let f = |t: f64| t;
        let s = quantize(f, 5);
        assert_eq!(s, vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    }

    #[test]
    fn quantize_emits_exactly_n() {
        let f = |t: f64| t * 10.0;
        let s = quantize(f, 3);
        assert_eq!(s, vec![0.0, 5.0, 10.0]);
    }

    #[test]
    #[should_panic(expected = ">= 2")]
    fn quantize_n_one_panics() {
        let _ = quantize(|t| t, 1);
    }

    #[test]
    fn piecewise_three_numbers() {
        // From d3: piecewise(interpolate, [0, 2, 10]):
        //   i(-1) = -4, i(0) = 0, i(0.19) = 0.76, i(0.21) = 0.84,
        //   i(0.5) = 2, i(0.75) = 6, i(1) = 10
        let values = vec![0.0_f64, 2.0, 10.0];
        let i = piecewise(&values, |&a, &b| {
            Box::new(move |t: f64| a * (1.0 - t) + b * t)
        });
        // The d3 piecewise extrapolates outside [0,1] using the endpoint
        // segments — that's why i(-1) = -4 (segment 0 with t' = -2):
        //   segment 0 covers [0, 0.5], endpoints (0, 2). t=-1 -> scaled=-2,
        //   i=0 (clamped), t' = -2 - 0 = -2. lerp(0, 2, -2) = -4.
        let assert_close = |actual: f64, expected: f64| {
            assert!((actual - expected).abs() < 1e-9, "{actual} vs {expected}");
        };
        assert_close(i(-1.0), -4.0);
        assert_close(i(0.0), 0.0);
        assert_close(i(0.19), 0.76);
        assert_close(i(0.21), 0.84);
        assert_close(i(0.5), 2.0);
        assert_close(i(0.75), 6.0);
        assert_close(i(1.0), 10.0);
    }

    #[test]
    #[should_panic(expected = "at least 2")]
    fn piecewise_one_value_panics() {
        let _ = piecewise(&[0.0_f64], |&a, &b| {
            Box::new(move |t: f64| a * (1.0 - t) + b * t)
        });
    }
}
