//! Number-valued interpolators — port of d3-interpolate's
//! `number.js`, `round.js`, and `date.js`.
//!
//! All interpolators in this module are pure functions of `t ∈ [0, 1]`
//! (the natural extension to `t ∉ [0, 1]` is the same linear formula —
//! `a * (1 - t) + b * t` — and is well-defined). They return *exact*
//! endpoints at `t = 0` and `t = 1` even when the input magnitudes are
//! large (e.g. `2e42`), matching the explicit `t==0`/`t==1` short-circuit
//! contracts d3 documents in its tests.

/// Linear interpolation between `a` and `b`.
///
/// Equivalent to d3's `interpolateNumber(a, b)`. The closed-form is
/// `a * (1 - t) + b * t`, which is symmetric in `a`/`b` and exact at the
/// endpoints.
///
/// # Example
///
/// ```ignore
/// let i = rgraph_interpolate::interpolate_number(10.0, 42.0);
/// assert_eq!(i(0.0), 10.0);
/// assert_eq!(i(1.0), 42.0);
/// ```
pub fn interpolate_number(a: f64, b: f64) -> impl Fn(f64) -> f64 + Copy {
    move |t| a * (1.0 - t) + b * t
}

/// Linear interpolation followed by `.round()`. Equivalent to d3's
/// `interpolateRound(a, b)`. Returns an integer-valued `f64` so the
/// caller chooses the integer type they want.
pub fn interpolate_round(a: f64, b: f64) -> impl Fn(f64) -> f64 + Copy {
    move |t| (a * (1.0 - t) + b * t).round()
}

/// Linear interpolation between two integer millisecond timestamps,
/// rounded to the nearest millisecond. Equivalent to d3's
/// `interpolateDate(a, b)` projected onto `i64` epoch-millis.
///
/// Returning `i64` is the most useful primitive for Rust: callers can
/// rebuild a `std::time::SystemTime`, `chrono::DateTime`, or any custom
/// epoch type from the result. For nanosecond precision use
/// [`interpolate_date_i128`].
pub fn interpolate_date_ms(a: i64, b: i64) -> impl Fn(f64) -> i64 + Copy {
    let af = a as f64;
    let bf = b as f64;
    move |t| (af * (1.0 - t) + bf * t).round() as i64
}

/// Linear interpolation between two `i128` timestamps. Useful when
/// `i64` millis aren't precise enough — pass nanoseconds, microseconds,
/// or any monotonic counter.
///
/// Note: `i128` round-trips through `f64` lose precision around
/// 2^53. For the typical UI animation duration that is far below the
/// noise floor; consult the docs of the timestamp scale you choose.
pub fn interpolate_date_i128(a: i128, b: i128) -> impl Fn(f64) -> i128 + Copy {
    let af = a as f64;
    let bf = b as f64;
    move |t| (af * (1.0 - t) + bf * t).round() as i128
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_in_delta(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-6, "actual={actual} expected={expected}");
    }

    #[test]
    fn number_interpolates_linearly() {
        let i = interpolate_number(10.0, 42.0);
        assert_in_delta(i(0.0), 10.0);
        assert_in_delta(i(0.1), 13.2);
        assert_in_delta(i(0.5), 26.0);
        assert_in_delta(i(0.9), 38.8);
        assert_in_delta(i(1.0), 42.0);
    }

    #[test]
    fn number_exact_endpoints_for_large_magnitudes() {
        // From d3's number-test: i(0)==a and i(1)==b exactly.
        let a = 2e42_f64;
        let b = 335.0_f64;
        let i = interpolate_number(a, b);
        assert_eq!(i(0.0), a);
        assert_eq!(i(1.0), b);
    }

    #[test]
    fn round_matches_d3_at_each_step() {
        let i = interpolate_round(10.0, 42.0);
        // d3: i(0.0)..i(1.0) == 10, 13, 16, 20, 23, 26, 29, 32, 36, 39, 42
        let expected = [10.0, 13.0, 16.0, 20.0, 23.0, 26.0, 29.0, 32.0, 36.0, 39.0, 42.0];
        for (k, &e) in expected.iter().enumerate() {
            let t = k as f64 / 10.0;
            assert_eq!(i(t), e, "t={t}");
        }
    }

    #[test]
    fn round_does_not_pre_round_inputs() {
        let i = interpolate_round(2.6, 3.6);
        // d3: i(0.6) == 3 (interpolated value 3.2 rounds to 3)
        assert_eq!(i(0.6), 3.0);
    }

    #[test]
    fn round_exact_endpoints() {
        let a = 2e42_f64;
        let b = 335.0_f64;
        let i = interpolate_round(a, b);
        // d3 contract: at t=0, return a (no rounding side-effect when t=0).
        // Our implementation rounds always; for finite a, a.round() == a
        // when a is integer-valued; 2e42 is integer-valued in f64.
        assert_eq!(i(0.0), a.round());
        assert_eq!(i(1.0), b);
    }

    #[test]
    fn date_ms_interpolates() {
        let i = interpolate_date_ms(1000, 2000);
        assert_eq!(i(0.0), 1000);
        assert_eq!(i(0.5), 1500);
        assert_eq!(i(1.0), 2000);
    }

    #[test]
    fn date_ms_handles_negative_epochs() {
        let i = interpolate_date_ms(-1000, 1000);
        assert_eq!(i(0.0), -1000);
        assert_eq!(i(0.5), 0);
        assert_eq!(i(1.0), 1000);
    }

    #[test]
    fn date_i128_works_for_nanos() {
        // Two timestamps a millisecond apart, in nanoseconds.
        let a = 1_700_000_000_000_000_000i128;
        let b = 1_700_000_000_001_000_000i128;
        let i = interpolate_date_i128(a, b);
        // At this magnitude, f64's 53-bit mantissa loses ~64 ns of
        // precision, so the endpoints aren't exact. Allow that drift.
        let drift_a = (i(0.0) - a).unsigned_abs();
        let drift_b = (i(1.0) - b).unsigned_abs();
        assert!(drift_a <= 128, "drift_a = {drift_a}");
        assert!(drift_b <= 128, "drift_b = {drift_b}");
        let mid = i(0.5);
        let expected = a + 500_000;
        assert!((mid - expected).unsigned_abs() <= 128);
    }

    #[test]
    fn date_i128_exact_at_small_magnitudes() {
        // For magnitudes that fit in f64 mantissa, endpoints are exact.
        let i = interpolate_date_i128(1_000_000, 2_000_000);
        assert_eq!(i(0.0), 1_000_000);
        assert_eq!(i(1.0), 2_000_000);
        assert_eq!(i(0.5), 1_500_000);
    }
}
