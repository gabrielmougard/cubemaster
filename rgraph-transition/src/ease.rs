//! Easing functions, port of [d3-ease](https://github.com/d3/d3-ease).
//!
//! All easing functions accept a normalized progress `t` in `[0, 1]` and return
//! the eased value, also in `[0, 1]` for the standard easings (some easings
//! such as [`back_in`] and [`elastic_in`] briefly leave the range, by design).
//!
//! # Categories
//!
//! Every category provides three variants:
//!
//! * `*_in` — slow start, fast finish.
//! * `*_out` — fast start, slow finish.
//! * `*_in_out` — symmetric ease in, ease out.
//!
//! Plus a default alias matching the d3 export (e.g. `cubic` aliases
//! `cubic_in_out`, `bounce` aliases `bounce_out`, `elastic` aliases
//! `elastic_out`).
//!
//! # Customizable easings
//!
//! Three families take parameters and expose builder structs that precompute
//! coefficients at construction time:
//!
//! * [`Back`] — overshoot factor (default `1.70158`).
//! * [`Poly`] — exponent (default `3.0`, equivalent to `cubic`).
//! * [`Elastic`] — amplitude (default `1.0`) and period (default `0.3`).
//!
//! # Example
//!
//! ```ignore
//! use rgraph_transition::ease::{cubic_in_out, Back};
//! assert!((cubic_in_out(0.5) - 0.5).abs() < 1e-12);
//! let back = Back::new(2.0);
//! let _ = back.in_out(0.25);
//! ```

/// `2^(-10*x)` rescaled to `[0, 1]`. Mirrors d3's `tpmt(x)`. The constants
/// keep the curve passing exactly through `(0, 0)` and `(1, 1)`.
#[inline(always)]
fn tpmt(x: f64) -> f64 {
    // 2.0_f64.powf(-10.0 * x) -> exp2(-10 * x) is a smidge faster + same value.
    ((-10.0 * x).exp2() - 0.0009765625) * 1.0009775171065494
}

// ---------------------------------------------------------------------------
// linear
// ---------------------------------------------------------------------------

/// Linear easing: `linear(t) == t`.
#[inline]
pub fn linear(t: f64) -> f64 { t }

// ---------------------------------------------------------------------------
// quadratic
// ---------------------------------------------------------------------------

/// `t^2`.
#[inline]
pub fn quad_in(t: f64) -> f64 { t * t }

/// Reflection of [`quad_in`].
#[inline]
pub fn quad_out(t: f64) -> f64 { t * (2.0 - t) }

/// Symmetric quadratic in/out.
#[inline]
pub fn quad_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        (t * t) / 2.0
    } else {
        let t = t - 1.0; // matches `--t`
        (t * (2.0 - t) + 1.0) / 2.0
    }
}

/// Default quad easing alias — d3's `easeQuad` resolves to `easeQuadInOut`.
#[inline]
pub fn quad(t: f64) -> f64 { quad_in_out(t) }

// ---------------------------------------------------------------------------
// cubic
// ---------------------------------------------------------------------------

#[inline]
pub fn cubic_in(t: f64) -> f64 { t * t * t }

#[inline]
pub fn cubic_out(t: f64) -> f64 {
    let t = t - 1.0;
    t * t * t + 1.0
}

#[inline]
pub fn cubic_in_out(t: f64) -> f64 {
    let t2 = t * 2.0;
    if t2 <= 1.0 {
        (t2 * t2 * t2) / 2.0
    } else {
        let t = t2 - 2.0;
        (t * t * t + 2.0) / 2.0
    }
}

/// d3's `easeCubic` alias of `easeCubicInOut`.
#[inline]
pub fn cubic(t: f64) -> f64 { cubic_in_out(t) }

// ---------------------------------------------------------------------------
// poly (parameterized)
// ---------------------------------------------------------------------------

/// Polynomial easing parameterized by an arbitrary positive exponent.
#[derive(Copy, Clone, Debug)]
pub struct Poly {
    /// Exponent passed to `t.powf(e)`. Default `3.0`.
    pub exponent: f64,
}

impl Default for Poly {
    fn default() -> Self { Poly { exponent: 3.0 } }
}

impl Poly {
    /// Construct a [`Poly`] easing with the given exponent.
    #[inline]
    pub const fn new(exponent: f64) -> Self { Poly { exponent } }

    #[inline]
    pub fn in_(self, t: f64) -> f64 { t.powf(self.exponent) }

    #[inline]
    pub fn out(self, t: f64) -> f64 { 1.0 - (1.0 - t).powf(self.exponent) }

    #[inline]
    pub fn in_out(self, t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            t.powf(self.exponent) / 2.0
        } else {
            (2.0 - (2.0 - t).powf(self.exponent)) / 2.0
        }
    }
}

/// Free-function aliases using the default exponent (3).
#[inline]
pub fn poly_in(t: f64) -> f64 { Poly::default().in_(t) }
#[inline]
pub fn poly_out(t: f64) -> f64 { Poly::default().out(t) }
#[inline]
pub fn poly_in_out(t: f64) -> f64 { Poly::default().in_out(t) }
#[inline]
pub fn poly(t: f64) -> f64 { poly_in_out(t) }

// ---------------------------------------------------------------------------
// sin
// ---------------------------------------------------------------------------

const HALF_PI: f64 = core::f64::consts::FRAC_PI_2;

#[inline]
pub fn sin_in(t: f64) -> f64 {
    if t == 1.0 { 1.0 } else { 1.0 - (t * HALF_PI).cos() }
}

#[inline]
pub fn sin_out(t: f64) -> f64 { (t * HALF_PI).sin() }

#[inline]
pub fn sin_in_out(t: f64) -> f64 { (1.0 - (core::f64::consts::PI * t).cos()) / 2.0 }

#[inline]
pub fn sin(t: f64) -> f64 { sin_in_out(t) }

// ---------------------------------------------------------------------------
// exp
// ---------------------------------------------------------------------------

#[inline]
pub fn exp_in(t: f64) -> f64 { tpmt(1.0 - t) }

#[inline]
pub fn exp_out(t: f64) -> f64 { 1.0 - tpmt(t) }

#[inline]
pub fn exp_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 { tpmt(1.0 - t) / 2.0 } else { (2.0 - tpmt(t - 1.0)) / 2.0 }
}

#[inline]
pub fn exp(t: f64) -> f64 { exp_in_out(t) }

// ---------------------------------------------------------------------------
// circle
// ---------------------------------------------------------------------------

#[inline]
pub fn circle_in(t: f64) -> f64 { 1.0 - (1.0 - t * t).sqrt() }

#[inline]
pub fn circle_out(t: f64) -> f64 {
    let t = t - 1.0;
    (1.0 - t * t).sqrt()
}

#[inline]
pub fn circle_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        (1.0 - (1.0 - t * t).sqrt()) / 2.0
    } else {
        let t = t - 2.0;
        ((1.0 - t * t).sqrt() + 1.0) / 2.0
    }
}

#[inline]
pub fn circle(t: f64) -> f64 { circle_in_out(t) }

// ---------------------------------------------------------------------------
// bounce
// ---------------------------------------------------------------------------

const B1: f64 = 4.0 / 11.0;
const B2: f64 = 6.0 / 11.0;
const B3: f64 = 8.0 / 11.0;
const B4: f64 = 3.0 / 4.0;
const B5: f64 = 9.0 / 11.0;
const B6: f64 = 10.0 / 11.0;
const B7: f64 = 15.0 / 16.0;
const B8: f64 = 21.0 / 22.0;
const B9: f64 = 63.0 / 64.0;
const B0: f64 = 1.0 / B1 / B1;

#[inline]
pub fn bounce_out(t: f64) -> f64 {
    if t < B1 {
        B0 * t * t
    } else if t < B3 {
        let t = t - B2;
        B0 * t * t + B4
    } else if t < B6 {
        let t = t - B5;
        B0 * t * t + B7
    } else {
        let t = t - B8;
        B0 * t * t + B9
    }
}

#[inline]
pub fn bounce_in(t: f64) -> f64 { 1.0 - bounce_out(1.0 - t) }

#[inline]
pub fn bounce_in_out(t: f64) -> f64 {
    let t = t * 2.0;
    if t <= 1.0 {
        (1.0 - bounce_out(1.0 - t)) / 2.0
    } else {
        (bounce_out(t - 1.0) + 1.0) / 2.0
    }
}

/// d3's `easeBounce` alias of `easeBounceOut`.
#[inline]
pub fn bounce(t: f64) -> f64 { bounce_out(t) }

// ---------------------------------------------------------------------------
// back (parameterized)
// ---------------------------------------------------------------------------

/// Default overshoot factor used by [`back_in`] / [`back_out`] / [`back_in_out`].
pub const BACK_OVERSHOOT: f64 = 1.70158;

/// Back easing parameterized by an overshoot factor.
#[derive(Copy, Clone, Debug)]
pub struct Back {
    /// Overshoot factor `s`. Larger values produce more pronounced overshoot.
    pub overshoot: f64,
}

impl Default for Back {
    fn default() -> Self { Back { overshoot: BACK_OVERSHOOT } }
}

impl Back {
    #[inline]
    pub const fn new(overshoot: f64) -> Self { Back { overshoot } }

    #[inline]
    pub fn in_(self, t: f64) -> f64 {
        let s = self.overshoot;
        t * t * (s * (t - 1.0) + t)
    }

    #[inline]
    pub fn out(self, t: f64) -> f64 {
        let s = self.overshoot;
        let t = t - 1.0;
        t * t * ((t + 1.0) * s + t) + 1.0
    }

    #[inline]
    pub fn in_out(self, t: f64) -> f64 {
        let s = self.overshoot;
        let t = t * 2.0;
        if t < 1.0 {
            (t * t * ((s + 1.0) * t - s)) / 2.0
        } else {
            let t = t - 2.0;
            (t * t * ((s + 1.0) * t + s) + 2.0) / 2.0
        }
    }
}

#[inline]
pub fn back_in(t: f64) -> f64 { Back::default().in_(t) }
#[inline]
pub fn back_out(t: f64) -> f64 { Back::default().out(t) }
#[inline]
pub fn back_in_out(t: f64) -> f64 { Back::default().in_out(t) }
#[inline]
pub fn back(t: f64) -> f64 { back_in_out(t) }

// ---------------------------------------------------------------------------
// elastic (parameterized)
// ---------------------------------------------------------------------------

const TAU: f64 = 2.0 * core::f64::consts::PI;
/// Default amplitude used by [`elastic_in`]/[`elastic_out`]/[`elastic_in_out`].
pub const ELASTIC_AMPLITUDE: f64 = 1.0;
/// Default period used by [`elastic_in`]/[`elastic_out`]/[`elastic_in_out`].
pub const ELASTIC_PERIOD: f64 = 0.3;

/// Elastic easing parameterized by amplitude and period.
///
/// Like d3, the effective amplitude is clamped to `>= 1`. Coefficients are
/// pre-computed at construction so each evaluation is just two muls, an `exp2`
/// and a `sin`.
#[derive(Copy, Clone, Debug)]
pub struct Elastic {
    /// Effective amplitude after clamping. Always `>= 1`.
    pub amplitude: f64,
    /// User-supplied period, normalized by tau.
    period_norm: f64, // p / tau
    /// `asin(1/a) * (p/tau)` — precomputed phase offset.
    phase: f64,
}

impl Default for Elastic {
    fn default() -> Self {
        Elastic::with_amplitude_and_period(ELASTIC_AMPLITUDE, ELASTIC_PERIOD)
    }
}

impl Elastic {
    /// Construct a new elastic easing with the given amplitude and period.
    pub fn with_amplitude_and_period(amplitude: f64, period: f64) -> Self {
        let a = amplitude.max(1.0);
        let pn = period / TAU;
        let phase = (1.0 / a).asin() * pn;
        Elastic { amplitude: a, period_norm: pn, phase }
    }

    /// Override the amplitude, preserving the *original* user-facing period.
    /// Mirrors d3's `elasticIn.amplitude(a)` chain.
    #[inline]
    pub fn amplitude(self, amplitude: f64) -> Self {
        // Recover the user-facing period (= period_norm * tau) and rebuild.
        Self::with_amplitude_and_period(amplitude, self.period_norm * TAU)
    }

    /// Override the period.
    #[inline]
    pub fn period(self, period: f64) -> Self {
        Self::with_amplitude_and_period(self.amplitude, period)
    }

    #[inline]
    pub fn in_(self, t: f64) -> f64 {
        let t = t - 1.0;
        self.amplitude * tpmt(-t) * ((self.phase - t) / self.period_norm).sin()
    }

    #[inline]
    pub fn out(self, t: f64) -> f64 {
        1.0 - self.amplitude * tpmt(t) * ((t + self.phase) / self.period_norm).sin()
    }

    #[inline]
    pub fn in_out(self, t: f64) -> f64 {
        let t = t * 2.0 - 1.0;
        if t < 0.0 {
            (self.amplitude * tpmt(-t) * ((self.phase - t) / self.period_norm).sin()) / 2.0
        } else {
            (2.0 - self.amplitude * tpmt(t) * ((t + self.phase) / self.period_norm).sin()) / 2.0
        }
    }
}

#[inline]
pub fn elastic_in(t: f64) -> f64 { Elastic::default().in_(t) }
#[inline]
pub fn elastic_out(t: f64) -> f64 { Elastic::default().out(t) }
#[inline]
pub fn elastic_in_out(t: f64) -> f64 { Elastic::default().in_out(t) }
/// d3's `easeElastic` alias of `easeElasticOut`.
#[inline]
pub fn elastic(t: f64) -> f64 { elastic_out(t) }

// ---------------------------------------------------------------------------
// EaseFn type alias
// ---------------------------------------------------------------------------

/// Convenient alias for plain easing function pointers.
pub type EaseFn = fn(f64) -> f64;

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_in_delta(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "actual={actual} expected={expected}"
        );
    }

    /// Generic out wrapper from d3 test/generic.js
    fn out_of(ease_in: impl Fn(f64) -> f64) -> impl Fn(f64) -> f64 {
        move |t| 1.0 - ease_in(1.0 - t)
    }

    fn in_out_of(ease_in: impl Fn(f64) -> f64 + Copy) -> impl Fn(f64) -> f64 {
        move |t| {
            if t < 0.5 {
                ease_in(t * 2.0) / 2.0
            } else {
                (2.0 - ease_in((1.0 - t) * 2.0)) / 2.0
            }
        }
    }

    // ----- linear -----
    #[test]
    fn linear_passes_through() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(linear(t), t);
        }
    }

    // ----- quad -----
    #[test]
    fn quad_in_expected() {
        let expected = [0.0, 0.01, 0.04, 0.09, 0.16, 0.25, 0.36, 0.49, 0.64, 0.81, 1.0];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(quad_in(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn quad_out_matches_generic() {
        let g = out_of(quad_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(quad_out(t), g(t));
        }
    }
    #[test]
    fn quad_in_out_matches_generic() {
        let g = in_out_of(quad_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(quad_in_out(t), g(t));
        }
    }
    #[test]
    fn quad_alias_is_in_out() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_eq!(quad(t), quad_in_out(t));
        }
    }

    // ----- cubic -----
    #[test]
    fn cubic_in_expected() {
        let expected = [0.0, 0.001, 0.008, 0.027, 0.064, 0.125, 0.216, 0.343, 0.512, 0.729, 1.0];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(cubic_in(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn cubic_out_matches_generic() {
        let g = out_of(cubic_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(cubic_out(t), g(t));
        }
    }
    #[test]
    fn cubic_in_out_matches_generic() {
        let g = in_out_of(cubic_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(cubic_in_out(t), g(t));
        }
    }
    #[test]
    fn cubic_alias_is_in_out() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_eq!(cubic(t), cubic_in_out(t));
        }
    }

    // ----- poly -----
    #[test]
    fn poly_in_default_matches_cubic_in() {
        for i in 1..10 {
            let t = i as f64 / 10.0;
            assert_in_delta(poly_in(t), cubic_in(t));
        }
    }
    #[test]
    fn poly_in_2_5_expected() {
        let p = Poly::new(2.5);
        let expected = [
            0.000000, 0.003162, 0.017889, 0.049295, 0.101193, 0.176777, 0.278855,
            0.409963, 0.572433, 0.768433, 1.0,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(p.in_(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn poly_out_matches_generic() {
        let p = Poly::new(2.5);
        let g = out_of(|t| p.in_(t));
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(p.out(t), g(t));
        }
    }
    #[test]
    fn poly_in_out_matches_generic() {
        let p = Poly::new(2.5);
        let g = in_out_of(|t| p.in_(t));
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(p.in_out(t), g(t));
        }
    }

    // ----- sin -----
    #[test]
    fn sin_in_expected() {
        let expected = [
            0.0, 0.012312, 0.048943, 0.108993, 0.190983, 0.292893, 0.412215, 0.546010,
            0.690983, 0.843566, 1.0,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(sin_in(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn sin_out_matches_generic() {
        let g = out_of(sin_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(sin_out(t), g(t));
        }
    }
    #[test]
    fn sin_in_out_matches_generic() {
        let g = in_out_of(sin_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(sin_in_out(t), g(t));
        }
    }

    // ----- exp -----
    #[test]
    fn exp_in_expected() {
        let expected = [
            0.0, 0.000978, 0.002933, 0.006843, 0.014663, 0.030303, 0.061584, 0.124145,
            0.249267, 0.499511, 1.0,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(exp_in(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn exp_out_matches_generic() {
        let g = out_of(exp_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(exp_out(t), g(t));
        }
    }
    #[test]
    fn exp_in_out_matches_generic() {
        let g = in_out_of(exp_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(exp_in_out(t), g(t));
        }
    }

    // ----- circle -----
    #[test]
    fn circle_in_expected() {
        let expected = [
            0.0, 0.005013, 0.020204, 0.046061, 0.083485, 0.133975, 0.200000, 0.285857,
            0.400000, 0.564110, 1.0,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(circle_in(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn circle_out_matches_generic() {
        let g = out_of(circle_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(circle_out(t), g(t));
        }
    }
    #[test]
    fn circle_in_out_matches_generic() {
        let g = in_out_of(circle_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(circle_in_out(t), g(t));
        }
    }

    // ----- bounce -----
    #[test]
    fn bounce_in_expected() {
        let expected = [
            0.0, 0.011875, 0.060000, 0.069375, 0.227500, 0.234375, 0.090000, 0.319375,
            0.697500, 0.924375, 1.0,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(bounce_in(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn bounce_out_matches_generic() {
        let g = out_of(bounce_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(bounce_out(t), g(t));
        }
    }
    #[test]
    fn bounce_in_out_matches_generic() {
        let g = in_out_of(bounce_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(bounce_in_out(t), g(t));
        }
    }
    #[test]
    fn bounce_alias_is_out() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_eq!(bounce(t), bounce_out(t));
        }
    }

    // ----- back -----
    #[test]
    fn back_in_expected() {
        let expected = [
            0.0, -0.014314, -0.046451, -0.080200, -0.099352, -0.087698, -0.029028,
            0.092868, 0.294198, 0.591172, 1.0,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(back_in(i as f64 / 10.0), e);
        }
    }
    #[test]
    fn back_out_matches_generic() {
        let g = out_of(back_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(back_out(t), g(t));
        }
    }
    #[test]
    fn back_in_out_matches_generic() {
        let g = in_out_of(back_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(back_in_out(t), g(t));
        }
    }
    #[test]
    fn back_alias_is_in_out() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_eq!(back(t), back_in_out(t));
        }
    }
    #[test]
    fn back_with_overshoot() {
        let b = Back::new(2.0);
        // Just sanity-check endpoints + monotone cusp behavior
        assert!((b.in_(0.0)).abs() < 1e-12);
        assert_in_delta(b.in_(1.0), 1.0);
    }

    // ----- elastic -----
    #[test]
    fn elastic_in_expected() {
        let expected = [
            0.0, 0.000978, -0.001466, -0.003421, 0.014663, -0.015152, -0.030792,
            0.124145, -0.124633, -0.249756, 1.0,
        ];
        for (i, &e) in expected.iter().enumerate() {
            assert_in_delta(elastic_in(i as f64 / 10.0), e);
        }
    }

    #[test]
    fn elastic_amplitude_below_one_clamped() {
        // d3: elasticIn.amplitude(a)(t) is the same as elasticIn(t) if a <= 1
        let a_neg = Elastic::default().amplitude(-1.0);
        let a_small = Elastic::default().amplitude(0.4);
        assert_in_delta(a_neg.in_(0.1), elastic_in(0.1));
        assert_in_delta(a_small.in_(0.2), elastic_in(0.2));
    }

    #[test]
    fn elastic_in_amplitude_1_3_expected() {
        let e = Elastic::default().amplitude(1.3);
        let expected = [
            0.0, 0.000978, -0.003576, 0.001501, 0.014663, -0.036951, 0.013510, 0.124145,
            -0.303950, 0.109580, 1.0,
        ];
        for (i, &v) in expected.iter().enumerate() {
            assert_in_delta(e.in_(i as f64 / 10.0), v);
        }
    }

    #[test]
    fn elastic_in_amplitude_1_5_period_1_expected() {
        let e = Elastic::default().amplitude(1.5).period(1.0);
        let expected = [
            0.0, 0.000148, -0.002212, -0.009390, -0.021498, -0.030303, -0.009352,
            0.093642, 0.342077, 0.732374, 1.0,
        ];
        for (i, &v) in expected.iter().enumerate() {
            assert_in_delta(e.in_(i as f64 / 10.0), v);
        }
    }

    #[test]
    fn elastic_out_matches_generic() {
        let g = out_of(elastic_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(elastic_out(t), g(t));
        }
    }
    #[test]
    fn elastic_in_out_matches_generic() {
        let g = in_out_of(elastic_in);
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(elastic_in_out(t), g(t));
        }
    }
    #[test]
    fn elastic_alias_is_out() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            assert_in_delta(elastic(t), elastic_out(t));
        }
    }
}
