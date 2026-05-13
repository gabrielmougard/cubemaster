//! Internal helper for formatting numbers identically to JavaScript's
//! default `String(number)` coercion.
//!
//! `String(1)` produces `"1"`, not `"1.0"`. Rust's `format!("{}", 1.0)`
//! produces `"1"` *only when* the value has no fractional part — but
//! values like `1.5` round-trip to `"1.5"` which matches JS, so we can
//! reuse the default formatter for almost all cases.
//!
//! The caveat is integer-valued floats: `format!("{}", 1.0)` in Rust
//! actually outputs `"1"` (no trailing `.0`) since `1.0` rendered with
//! the default `Display` impl yields the shortest repr. So this helper
//! is currently a thin wrapper, but kept for parity / future tweaks.

#![allow(dead_code)]

use std::fmt::Write;

/// Format an `f64` the way JavaScript's `String(number)` does.
///
/// Used by the SVG path builders to keep output byte-identical with
/// the React-Flow / Svelte-Flow upstream so SSR snapshot diffs remain
/// clean across hydrations.
#[must_use]
pub(crate) fn js_num(n: f64) -> String {
    if n == 0.0 {
        // Rust's default formatter outputs "-0" for negative zero;
        // JS's String(-0) produces "0".
        return "0".to_string();
    }
    if !n.is_finite() {
        // JS String(Infinity) = "Infinity", String(-Infinity) = "-Infinity",
        // String(NaN) = "NaN". Match these.
        if n.is_nan() {
            return "NaN".to_string();
        }
        return if n.is_sign_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    // Rust's default `{}` formatter for f64 already uses the shortest
    // round-trip representation, which matches JS for most values
    // including integer-valued ones (`1.0` → `"1"`).
    let mut s = String::new();
    write!(s, "{n}").expect("writing into String never fails");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_have_no_decimals() {
        assert_eq!(js_num(1.0), "1");
        assert_eq!(js_num(-1.0), "-1");
        assert_eq!(js_num(100.0), "100");
    }

    #[test]
    fn fractions_are_preserved() {
        assert_eq!(js_num(1.5), "1.5");
        assert_eq!(js_num(-2.25), "-2.25");
    }

    #[test]
    fn negative_zero_collapses_to_zero() {
        assert_eq!(js_num(-0.0), "0");
        assert_eq!(js_num(0.0), "0");
    }

    #[test]
    fn special_values() {
        assert_eq!(js_num(f64::NAN), "NaN");
        assert_eq!(js_num(f64::INFINITY), "Infinity");
        assert_eq!(js_num(f64::NEG_INFINITY), "-Infinity");
    }
}
