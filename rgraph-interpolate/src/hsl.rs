//! HSL color-space interpolation — port of d3-interpolate's `hsl.js`.

use rgraph_color::{Hsl, hsl_from, parse};

use crate::StringInterp;
use crate::color_helpers::{hue, nogamma};

fn parse_hsl(s: &str) -> Hsl {
    match parse(s) {
        Some(c) => hsl_from(&c),
        None => Hsl::empty(),
    }
}

/// HSL interpolation taking the **shortest** hue path (≤ 180° in either
/// direction). Matches d3's default `interpolateHsl(a, b)`.
pub fn interpolate_hsl_str(a: &str, b: &str) -> StringInterp {
    interpolate_hsl(parse_hsl(a), parse_hsl(b))
}

/// HSL interpolation taking the shortest hue path, between pre-parsed
/// [`Hsl`] colors.
pub fn interpolate_hsl(a: Hsl, b: Hsl) -> StringInterp {
    interpolate_hsl_inner(a, b, false)
}

/// HSL interpolation taking the **longer** (counter-clockwise/clockwise
/// without shortest-path normalisation) hue path. Matches d3's
/// `interpolateHslLong`.
pub fn interpolate_hsl_long_str(a: &str, b: &str) -> StringInterp {
    interpolate_hsl_long(parse_hsl(a), parse_hsl(b))
}

/// "Long" HSL interpolation between pre-parsed colors.
pub fn interpolate_hsl_long(a: Hsl, b: Hsl) -> StringInterp {
    interpolate_hsl_inner(a, b, true)
}

fn interpolate_hsl_inner(a: Hsl, b: Hsl, long: bool) -> StringInterp {
    let h = if long { nogamma(a.h, b.h) } else { hue(a.h, b.h) };
    let s = nogamma(a.s, b.s);
    let l = nogamma(a.l, b.l);
    let opacity = nogamma(a.opacity, b.opacity);
    Box::new(move |t| {
        let mixed = Hsl::new(h(t), s(t), l(t), opacity(t));
        // d3's HSL toString → RGB string. rgraph-color's Display does the same.
        format!("{mixed}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_match_native() {
        let i = interpolate_hsl_str("steelblue", "brown");
        let s0 = format!("{}", parse_hsl("steelblue"));
        let s1 = format!("{}", parse_hsl("brown"));
        assert_eq!(i(0.0), s0);
        assert_eq!(i(1.0), s1);
    }

    #[test]
    fn long_vs_short_differ_for_far_hues() {
        // Two colors with hues that are NOT a multiple of 120° or
        // exactly 180° apart, so short and long paths actually diverge.
        // red (h=0) → magenta-ish (h=300), short path goes via 330° = -30°
        // (equivalent to going backward 60°), long path goes forward 300°.
        let i_short = interpolate_hsl_str("#f00", "#f0f");
        let i_long = interpolate_hsl_long_str("#f00", "#f0f");
        assert_ne!(i_short(0.5), i_long(0.5));
    }

    #[test]
    fn opposite_hues_180_paths_agree() {
        // For exactly 180°-apart hues there is no "shorter" direction;
        // both paths yield the same midpoint.
        let i_short = interpolate_hsl_str("red", "cyan");
        let i_long = interpolate_hsl_long_str("red", "cyan");
        assert_eq!(i_short(0.5), i_long(0.5));
    }
}
