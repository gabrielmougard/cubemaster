//! HCL color-space interpolation — port of d3-interpolate's `hcl.js`.
//!
//! HCL is the cylindrical sibling of Lab: hue (H, degrees), chroma (C),
//! lightness (L). Interpolating in HCL produces perceptually smoother
//! gradients than RGB or HSL for many palettes.

use rgraph_color::{Hcl, hcl_from, parse};

use crate::StringInterp;
use crate::color_helpers::{hue, nogamma};

fn parse_hcl(s: &str) -> Hcl {
    match parse(s) {
        Some(c) => hcl_from(&c),
        None => Hcl::empty(),
    }
}

/// HCL interpolation taking the **shortest** hue path.
pub fn interpolate_hcl_str(a: &str, b: &str) -> StringInterp {
    interpolate_hcl(parse_hcl(a), parse_hcl(b))
}

/// HCL interpolation taking the shortest hue path, between pre-parsed
/// [`Hcl`] colors.
pub fn interpolate_hcl(a: Hcl, b: Hcl) -> StringInterp {
    interpolate_hcl_inner(a, b, false)
}

/// HCL interpolation taking the **long** hue path (no shortest-path
/// normalisation). Mirrors d3's `interpolateHclLong`.
pub fn interpolate_hcl_long_str(a: &str, b: &str) -> StringInterp {
    interpolate_hcl_long(parse_hcl(a), parse_hcl(b))
}

/// "Long" HCL interpolation between pre-parsed colors.
pub fn interpolate_hcl_long(a: Hcl, b: Hcl) -> StringInterp {
    interpolate_hcl_inner(a, b, true)
}

fn interpolate_hcl_inner(a: Hcl, b: Hcl, long: bool) -> StringInterp {
    let h = if long { nogamma(a.h, b.h) } else { hue(a.h, b.h) };
    let c = nogamma(a.c, b.c);
    let l = nogamma(a.l, b.l);
    let opacity = nogamma(a.opacity, b.opacity);
    Box::new(move |t| {
        let mixed = Hcl::new(h(t), c(t), l(t), opacity(t));
        format!("{mixed}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_match_input() {
        let i = interpolate_hcl_str("steelblue", "brown");
        let s0 = format!("{}", parse_hcl("steelblue"));
        let s1 = format!("{}", parse_hcl("brown"));
        assert_eq!(i(0.0), s0);
        assert_eq!(i(1.0), s1);
    }

    #[test]
    fn long_and_short_differ_for_acute_hue_diff() {
        let i_short = interpolate_hcl_str("red", "blue");
        let i_long = interpolate_hcl_long_str("red", "blue");
        let m1 = i_short(0.5);
        let m2 = i_long(0.5);
        // For most input pairs these will differ.
        assert_ne!(m1, m2);
    }
}
