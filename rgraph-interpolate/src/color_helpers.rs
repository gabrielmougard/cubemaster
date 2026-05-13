//! Internal helpers shared by color-channel interpolators.
//!
//! Mirrors d3-interpolate's `color.js`: a small toolkit of `linear` /
//! `exponential` / `nogamma` / `hue` / `gamma` factories that the typed
//! color interpolators (`rgb`, `hsl`, `lab`, `hcl`, `cubehelix`) compose
//! together.
//!
//! NaN handling follows d3 exactly: when one endpoint is NaN, the
//! interpolator collapses to the *other* endpoint as a constant. This is
//! what gives RGB interpolation of `null` → `rgb(20, 40, 60)` the same
//! result as `rgb(20, 40, 60)` would (see d3's rgb-test).

/// Boxed scalar interpolator. Aliased to keep nested-generic signatures
/// readable across the color-helper API.
pub type ChannelFn = Box<dyn Fn(f64) -> f64>;

/// Boxed factory producing a `ChannelFn` from `(a, b)` endpoints.
pub type ChannelFactory = Box<dyn Fn(f64, f64) -> ChannelFn>;

/// `t -> a + t * d` — straight linear interpolation written in
/// "delta form" so it composes well with `nogamma`.
#[inline]
fn linear(a: f64, d: f64) -> ChannelFn {
    Box::new(move |t| a + t * d)
}

/// `t -> (a + t * b)^(1/y)` — gamma-corrected interpolation. d3
/// pre-computes `pow(a, y)` and `pow(b, y) - pow(a, y)` once so each
/// evaluation is just one `pow`, one mul, one add.
#[inline]
fn exponential(a: f64, b: f64, y: f64) -> ChannelFn {
    let a_pow = a.powf(y);
    let b_pow_diff = b.powf(y) - a_pow;
    let inv_y = 1.0 / y;
    Box::new(move |t| (a_pow + t * b_pow_diff).powf(inv_y))
}

/// Constant returning `x` (used when one endpoint is NaN).
#[inline]
fn constant(x: f64) -> ChannelFn {
    Box::new(move |_| x)
}

/// d3's `nogamma`: linear interpolation with NaN fallback.
///
/// If `b - a` is non-zero (i.e. both finite and different), interpolates
/// linearly. Otherwise, returns whichever of `a` / `b` is finite.
pub fn nogamma(a: f64, b: f64) -> ChannelFn {
    let d = b - a;
    if d != 0.0 && !d.is_nan() {
        linear(a, d)
    } else if a.is_nan() {
        constant(b)
    } else {
        constant(a)
    }
}

/// d3's `gamma(y)` factory: returns a `(a, b) -> Fn(f64) -> f64`
/// closure that interpolates with the chosen gamma. `gamma(1.0)` is
/// equivalent to `nogamma`.
pub fn gamma(y: f64) -> ChannelFactory {
    if y == 1.0 {
        Box::new(|a, b| nogamma(a, b))
    } else {
        Box::new(move |a, b| {
            let d = b - a;
            if d != 0.0 && !d.is_nan() {
                exponential(a, b, y)
            } else if a.is_nan() {
                constant(b)
            } else {
                constant(a)
            }
        })
    }
}

/// Hue interpolator: same as `nogamma` but takes the shortest path
/// around the 360° cycle.
///
/// d3's `hue(a, b)`: if the absolute difference exceeds 180°, it offsets
/// `b` by ±360° to choose the short path. NaN handling matches `nogamma`.
pub fn hue(a: f64, b: f64) -> ChannelFn {
    let d = b - a;
    if d != 0.0 && !d.is_nan() {
        // Shortest-path normalisation: if |d| > 180, shift by 360.
        let d = if !(-180.0..=180.0).contains(&d) {
            d - 360.0 * (d / 360.0).round()
        } else {
            d
        };
        linear(a, d)
    } else if a.is_nan() {
        constant(b)
    } else {
        constant(a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "{a} vs {b}");
    }

    #[test]
    fn nogamma_linear_when_distinct() {
        let f = nogamma(0.0, 100.0);
        close(f(0.0), 0.0);
        close(f(0.5), 50.0);
        close(f(1.0), 100.0);
    }

    #[test]
    fn nogamma_constant_when_same() {
        let f = nogamma(7.0, 7.0);
        close(f(0.0), 7.0);
        close(f(0.5), 7.0);
        close(f(1.0), 7.0);
    }

    #[test]
    fn nogamma_constant_when_a_is_nan() {
        let f = nogamma(f64::NAN, 42.0);
        close(f(0.5), 42.0);
    }

    #[test]
    fn nogamma_constant_when_b_is_nan() {
        let f = nogamma(42.0, f64::NAN);
        close(f(0.5), 42.0);
    }

    #[test]
    fn gamma_one_is_linear() {
        let mk = gamma(1.0);
        let f = mk(0.0, 100.0);
        close(f(0.5), 50.0);
    }

    #[test]
    fn gamma_three_curves() {
        let mk = gamma(3.0);
        let f = mk(0.0, 1.0);
        // For y=3: f(t) = (0 + t*(1 - 0))^(1/3) = t^(1/3).
        close(f(0.0), 0.0);
        close(f(1.0), 1.0);
        close(f(0.125), 0.125_f64.powf(1.0 / 3.0));
    }

    #[test]
    fn hue_uses_short_path_forward() {
        // a=350, b=10, naive d=-340 -> short path d = -340 + 360 = +20
        let f = hue(350.0, 10.0);
        // At t=0.5, expect 350 + 0.5 * 20 = 360 (mod 360 == 0). Note: hue()
        // does NOT mod 360 — that's the caller's job. We just check the
        // raw value.
        close(f(0.5), 360.0);
    }

    #[test]
    fn hue_uses_short_path_backward() {
        // a=10, b=350, naive d=+340 -> short path d = -20
        let f = hue(10.0, 350.0);
        close(f(0.5), 0.0);
    }

    #[test]
    fn hue_within_180_uses_direct_path() {
        let f = hue(100.0, 200.0);
        close(f(0.5), 150.0);
    }

    #[test]
    fn hue_nan_fallbacks() {
        close(hue(f64::NAN, 42.0)(0.5), 42.0);
        close(hue(42.0, f64::NAN)(0.5), 42.0);
    }
}
