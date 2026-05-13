//! String interpolation — port of d3-interpolate's `string.js`.
//!
//! d3's string interpolator scans both `a` and `b` for matching numeric
//! tokens, builds an interpolator for each pair, and weaves the result
//! back together with `b`'s static text. The result is a function that
//! produces a fresh string per call:
//!
//! * Numbers in `b` that don't have a counterpart in `a` are passed
//!   through unchanged.
//! * If both strings contain identical numeric tokens, the token is kept
//!   as-is (same identity — d3 treats `1000` and `1e3` as identical at
//!   value level but emits the source-text-of-`b` form on equality).
//! * Strings with no numeric content collapse to a constant returning
//!   `b`.

use crate::number::interpolate_number;

/// One entry in the per-call number-interpolator list: the `pieces` slot
/// to fill, and the boxed interpolator producing the value.
type SlotInterp = (usize, Box<dyn Fn(f64) -> f64>);

/// Numeric-aware string interpolator.
///
/// Returns a closure that produces a string by linearly interpolating
/// each pair of matched numeric tokens and reusing `b`'s non-numeric
/// segments verbatim.
///
/// Equivalent to d3's `interpolateString(a, b)`. Endpoint behavior:
///
/// * `t = 0` returns a string equivalent to `a` *projected onto `b`'s
///   shape* — i.e. it has `b`'s template with `a`'s numeric values where
///   they paired, and `b`'s values where they didn't.
/// * `t = 1` returns `b`.
pub fn interpolate_string(a: &str, b: &str) -> Box<dyn Fn(f64) -> String> {
    // 1) Scan `a` and `b` in lock-step for numeric tokens, building:
    //    - `pieces`: a Vec<Piece> alternating literal text and number
    //      placeholders.
    //    - `interps`: the per-placeholder linear interpolators.
    let a_nums = scan_numbers(a);
    let b_nums = scan_numbers(b);

    let mut pieces: Vec<Piece> = Vec::new();
    let mut interps: Vec<SlotInterp> = Vec::new();

    let n = a_nums.len().min(b_nums.len());
    let mut bi: usize = 0; // cursor into `b`
    for k in 0..n {
        let am = &a_nums[k];
        let bm = &b_nums[k];
        // 1a) Static text in b before this number.
        if bm.start > bi {
            push_text(&mut pieces, &b[bi..bm.start]);
        }
        // 1b) The numeric token: identical -> keep as-is, else placeholder.
        if am.value == bm.value {
            push_text(&mut pieces, &b[bm.start..bm.end]);
        } else {
            // Placeholder slot we'll fill at runtime.
            let slot = pieces.len();
            pieces.push(Piece::Placeholder);
            interps.push((slot, Box::new(interpolate_number(am.value, bm.value))));
        }
        bi = bm.end;
    }
    // 2) Trailing tail of `b`.
    if bi < b.len() {
        push_text(&mut pieces, &b[bi..]);
    }

    // 3) Optimize the trivial cases.
    if pieces.is_empty() {
        // Both inputs had no numeric content (or empty) -> constant b.
        let b_owned = b.to_owned();
        return Box::new(move |_| b_owned.clone());
    }

    if pieces.len() == 1 {
        return match pieces.into_iter().next().unwrap() {
            Piece::Text(s) => Box::new(move |_| s.clone()),
            Piece::Placeholder => {
                // Single number, no surrounding text. Format the
                // interpolated value using the same shortest-round-trip
                // form that d3 (JS String coercion) produces.
                let (_, f) = interps.into_iter().next().unwrap();
                Box::new(move |t| format_f64(f(t)))
            }
        };
    }

    // 4) General path: stitch all pieces together each call.
    Box::new(move |t| {
        let mut out = String::new();
        // Borrow each placeholder's interpolator-produced string lazily
        // by index. interps' first field is the index into `pieces`.
        let mut interp_iter = interps.iter().peekable();
        for (i, p) in pieces.iter().enumerate() {
            match p {
                Piece::Text(s) => out.push_str(s),
                Piece::Placeholder => {
                    // Fast-forward to the matching interpolator.
                    while let Some((idx, _)) = interp_iter.peek() {
                        if *idx == i { break; }
                        interp_iter.next();
                    }
                    let (_, f) = interp_iter.next().expect("placeholder w/o interp");
                    out.push_str(&format_f64(f(t)));
                }
            }
        }
        out
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Piece {
    Text(String),
    Placeholder,
}

/// Coalesce consecutive `Text(..)` chunks while pushing — matches d3's
/// `if (s[i]) s[i] += bs;` optimisation.
fn push_text(pieces: &mut Vec<Piece>, s: &str) {
    if s.is_empty() { return; }
    if let Some(Piece::Text(prev)) = pieces.last_mut() {
        prev.push_str(s);
    } else {
        pieces.push(Piece::Text(s.to_owned()));
    }
}

/// One scanned numeric token.
#[derive(Debug, Clone, Copy)]
struct NumToken {
    /// Byte offset of the token's first character in the source string.
    start: usize,
    /// Byte offset just past the token.
    end: usize,
    /// Parsed value.
    value: f64,
}

/// Scan a string for numeric tokens matching d3's regex
/// `/[-+]?(?:\d+\.?\d*|\.?\d+)(?:[eE][-+]?\d+)?/g`.
///
/// Faithful: leading sign, integer-with-optional-decimal OR
/// decimal-only-with-optional-leading-dot, optional exponent.
fn scan_numbers(s: &str) -> Vec<NumToken> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if let Some((start, end)) = find_number(bytes, i) {
            // Parse the matched slice.
            // SAFETY: the matched range is ASCII per `find_number`'s rules.
            let txt = std::str::from_utf8(&bytes[start..end]).unwrap();
            // d3 uses JS Number coercion; Rust's f64::from_str is
            // close enough for the patterns matched (decimal, exponent).
            // Edge case: a lone "-" or "+" with no digits cannot match
            // because find_number rejects it.
            if let Ok(v) = txt.parse::<f64>() {
                out.push(NumToken { start, end, value: v });
            } else {
                // Trailing-dot forms like "1." parse fine with f64::from_str;
                // patterns we'd reject are unlikely. Skip the byte.
                i = end;
                continue;
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// Find the next numeric token starting at or after `from`. Returns
/// `(start, end)` byte offsets, or `None` if no match.
fn find_number(b: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut i = from;
    while i < b.len() {
        // Try to match a number starting at i.
        if let Some(end) = match_number_at(b, i) {
            return Some((i, end));
        }
        i += 1;
    }
    None
}

/// Match a number anchored at `i`. Returns the index just past the
/// match, or `None`.
fn match_number_at(b: &[u8], i: usize) -> Option<usize> {
    let mut k = i;
    let n = b.len();
    // Optional sign — but only if followed by a number-ish thing.
    let signed = if k < n && (b[k] == b'+' || b[k] == b'-') {
        k += 1;
        true
    } else { false };

    let mut digits_before_dot = false;
    while k < n && b[k].is_ascii_digit() {
        digits_before_dot = true;
        k += 1;
    }
    let mut digits_after_dot = false;
    let saw_dot = if k < n && b[k] == b'.' {
        k += 1;
        while k < n && b[k].is_ascii_digit() {
            digits_after_dot = true;
            k += 1;
        }
        true
    } else { false };

    // Per d3's regex: `\d+\.?\d*` OR `\.?\d+`. So we need either:
    //   - at least one digit before the dot (digits_before_dot), OR
    //   - the dot followed by at least one digit (saw_dot && digits_after_dot)
    if !(digits_before_dot || (saw_dot && digits_after_dot)) {
        return None;
    }
    // Special case: bare "-" or "+" should not match.
    if signed && k - i == 1 { return None; }

    // Optional exponent: [eE][-+]?\d+
    if k < n && (b[k] == b'e' || b[k] == b'E') {
        let exp_start = k;
        k += 1;
        if k < n && (b[k] == b'+' || b[k] == b'-') { k += 1; }
        let exp_digits_start = k;
        while k < n && b[k].is_ascii_digit() { k += 1; }
        // If we consumed an `e` but no digits follow, back off — d3's
        // regex requires `\d+` after the exponent prefix.
        if k == exp_digits_start { k = exp_start; }
    }

    Some(k)
}

/// Format a float the way d3 does (effectively JavaScript's String
/// coercion: shortest round-trippable form). Rust's default `Display` for
/// `f64` does this too.
fn format_f64(v: f64) -> String {
    format!("{}", v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_numbers_interpolate() {
        let i = interpolate_string(" 10/20 30", "50/10 100 ");
        assert_eq!(i(0.2), "18/18 44 ");
        assert_eq!(i(0.4), "26/16 58 ");
    }

    #[test]
    fn preserves_static_text_in_b() {
        let i = interpolate_string(" 10/20 30", "50/10 foo ");
        assert_eq!(i(0.2), "18/18 foo ");
        assert_eq!(i(0.4), "26/16 foo ");
    }

    #[test]
    fn unmatched_b_numbers_are_constant() {
        let i = interpolate_string(" 10/20 foo", "50/10 100 ");
        assert_eq!(i(0.2), "18/18 100 ");
        assert_eq!(i(0.4), "26/16 100 ");
    }

    #[test]
    fn equal_numbers_in_both_kept_as_is() {
        let i = interpolate_string(" 10/20 100 20", "50/10 100, 20 ");
        assert_eq!(i(0.2), "18/18 100, 20 ");
        assert_eq!(i(0.4), "26/16 100, 20 ");
    }

    #[test]
    fn trailing_dot_decimal() {
        let i = interpolate_string("1.", "2.");
        assert_eq!(i(0.5), "1.5");
    }

    #[test]
    fn exponent_notation_interpolates() {
        // d3: interpolateString("1e+3","1e+4")(0.5) = "5500"
        // because 1e+3 -> 1000, 1e+4 -> 10000, midpoint = 5500.
        let i = interpolate_string("1e+3", "1e+4");
        assert_eq!(i(0.5), "5500");
    }

    #[test]
    fn negative_exponent_notation() {
        let i = interpolate_string("1e-3", "1e-4");
        assert_eq!(i(0.5), "0.00055");
    }

    #[test]
    fn signed_decimal_exponent() {
        let i = interpolate_string("-1.e-3", "-1.e-4");
        assert_eq!(i(0.5), "-0.00055");
    }

    #[test]
    fn leading_plus_dropped_in_output() {
        // d3 normalises "+1.e-3" to interpolate as 0.001; emitted
        // text uses Rust's f64 Display which is the same shortest-form.
        let i = interpolate_string("+1.e-3", "+1.e-4");
        assert_eq!(i(0.5), "0.00055");
    }

    #[test]
    fn dot_prefix_decimal() {
        let i = interpolate_string(".1e-2", ".1e-3");
        assert_eq!(i(0.5), "0.00055");
    }

    #[test]
    fn no_numbers_returns_b() {
        assert_eq!(interpolate_string("foo", "bar")(0.5), "bar");
        assert_eq!(interpolate_string("foo", "")(0.5), "");
        assert_eq!(interpolate_string("", "bar")(0.5), "bar");
        assert_eq!(interpolate_string("", "")(0.5), "");
    }

    #[test]
    fn equal_value_different_format_uses_b_form() {
        // d3: same numeric value but different text -> emit b's form
        // when the values match exactly. "1000" vs "1e3" both equal
        // 1000.0 numerically.
        // Note: we check value-level equality, so this preserves d3's
        // behavior of emitting the b form.
        let i = interpolate_string("top: 1000px;", "top: 1e3px;");
        assert_eq!(i(0.5), "top: 1e3px;");
        let i = interpolate_string("top: 1e3px;", "top: 1000px;");
        assert_eq!(i(0.5), "top: 1000px;");
    }

    #[test]
    fn scan_numbers_finds_signed() {
        // Note: we deliberately avoid 3.14 here — clippy flags it as an
        // approximation of π; pick an unrelated value.
        let toks = scan_numbers("a -5 +3.25 .5e2 1.");
        let vals: Vec<f64> = toks.iter().map(|t| t.value).collect();
        assert_eq!(vals, vec![-5.0, 3.25, 50.0, 1.0]);
    }

    #[test]
    fn scan_numbers_skips_lone_signs() {
        let toks = scan_numbers("- + a");
        assert!(toks.is_empty());
    }

    #[test]
    fn scan_numbers_skips_bad_exponent() {
        // "5e" should match "5" only (exponent backed off).
        let toks = scan_numbers("5e");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].value, 5.0);
    }
}
