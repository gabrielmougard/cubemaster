//! Tests — faithful port of d3-color's test suite.
//!
//! The numeric expectations come from running d3-color v3 on the relevant
//! inputs; preserving them lets us assert exact behavioural parity.

use crate::*;

// ---------------------------------------------------------------------------
// Approximate-equality helpers (mirrors test/asserts.js).
// ---------------------------------------------------------------------------

const EPS: f64 = 1e-6;

fn nan_eq(a: f64, b: f64) -> bool {
    if b.is_nan() { a.is_nan() } else { (a - b).abs() <= EPS }
}

#[track_caller]
fn assert_rgb_eq(actual: Rgb, r: f64, g: f64, b: f64, opacity: f64) {
    let ok = exact_or_nan(actual.r, r)
        && exact_or_nan(actual.g, g)
        && exact_or_nan(actual.b, b)
        && exact_or_nan(actual.opacity, opacity);
    assert!(ok, "rgb_eq mismatch: got ({}, {}, {}, {}), want ({}, {}, {}, {})",
        actual.r, actual.g, actual.b, actual.opacity, r, g, b, opacity);
}

fn exact_or_nan(a: f64, b: f64) -> bool {
    if b.is_nan() { a.is_nan() } else { a == b }
}

#[track_caller]
fn assert_rgb_approx(actual: Rgb, r: f64, g: f64, b: f64, opacity: f64) {
    let ok = round_or_nan(actual.r, r)
        && round_or_nan(actual.g, g)
        && round_or_nan(actual.b, b)
        && exact_or_nan(actual.opacity, opacity);
    assert!(ok, "rgb_approx mismatch: got ({}, {}, {}, {}), want ({}, {}, {}, {})",
        actual.r, actual.g, actual.b, actual.opacity, r, g, b, opacity);
}

fn round_or_nan(a: f64, b: f64) -> bool {
    if b.is_nan() { a.is_nan() } else { a.round() == b.round() }
}

#[track_caller]
fn assert_hsl_eq(actual: Hsl, h: f64, s: f64, l: f64, opacity: f64) {
    let ok = nan_eq(actual.h, h)
        && nan_eq(actual.s, s)
        && nan_eq(actual.l, l)
        && exact_or_nan(actual.opacity, opacity);
    assert!(ok, "hsl_eq mismatch: got ({}, {}, {}, {}), want ({}, {}, {}, {})",
        actual.h, actual.s, actual.l, actual.opacity, h, s, l, opacity);
}

#[track_caller]
fn assert_lab_eq(actual: Lab, l: f64, a: f64, b: f64, opacity: f64) {
    let ok = nan_eq(actual.l, l)
        && nan_eq(actual.a, a)
        && nan_eq(actual.b, b)
        && exact_or_nan(actual.opacity, opacity);
    assert!(ok, "lab_eq mismatch: got ({}, {}, {}, {}), want ({}, {}, {}, {})",
        actual.l, actual.a, actual.b, actual.opacity, l, a, b, opacity);
}

#[track_caller]
fn assert_hcl_eq(actual: Hcl, h: f64, c: f64, l: f64, opacity: f64) {
    let ok = nan_eq(actual.h, h)
        && nan_eq(actual.c, c)
        && nan_eq(actual.l, l)
        && exact_or_nan(actual.opacity, opacity);
    assert!(ok, "hcl_eq mismatch: got ({}, {}, {}, {}), want ({}, {}, {}, {})",
        actual.h, actual.c, actual.l, actual.opacity, h, c, l, opacity);
}

// ===========================================================================
// color() parser tests
// ===========================================================================

#[test]
fn color_parses_named() {
    assert_rgb_approx(parse("moccasin").unwrap().rgb(), 255.0, 228.0, 181.0, 1.0);
    assert_rgb_approx(parse("aliceblue").unwrap().rgb(), 240.0, 248.0, 255.0, 1.0);
    assert_rgb_approx(parse("yellow").unwrap().rgb(), 255.0, 255.0, 0.0, 1.0);
    assert_rgb_approx(parse("rebeccapurple").unwrap().rgb(), 102.0, 51.0, 153.0, 1.0);
    assert_rgb_approx(parse("transparent").unwrap().rgb(), f64::NAN, f64::NAN, f64::NAN, 0.0);
}

#[test]
fn color_parses_6_digit_hex() {
    assert_rgb_approx(parse("#abcdef").unwrap().rgb(), 171.0, 205.0, 239.0, 1.0);
}

#[test]
fn color_parses_3_digit_hex() {
    assert_rgb_approx(parse("#abc").unwrap().rgb(), 170.0, 187.0, 204.0, 1.0);
}

#[test]
fn color_rejects_7_digit_hex() {
    assert!(parse("#abcdef3").is_none());
}

#[test]
fn color_parses_8_digit_hex() {
    assert_rgb_approx(parse("#abcdef33").unwrap().rgb(), 171.0, 205.0, 239.0, 0.2);
}

#[test]
fn color_parses_4_digit_hex() {
    assert_rgb_approx(parse("#abc3").unwrap().rgb(), 170.0, 187.0, 204.0, 0.2);
}

#[test]
fn color_parses_rgb_integer() {
    assert_rgb_approx(parse("rgb(12,34,56)").unwrap().rgb(), 12.0, 34.0, 56.0, 1.0);
}

#[test]
fn color_parses_rgba_integer() {
    assert_rgb_approx(parse("rgba(12,34,56,0.4)").unwrap().rgb(), 12.0, 34.0, 56.0, 0.4);
}

#[test]
fn color_parses_rgb_percent() {
    assert_rgb_approx(parse("rgb(12%,34%,56%)").unwrap().rgb(), 31.0, 87.0, 143.0, 1.0);
    let c = parse("rgb(100%,100%,100%)").unwrap();
    assert_rgb_eq(c.rgb(), 255.0, 255.0, 255.0, 1.0);
}

#[test]
fn color_parses_rgba_percent() {
    assert_rgb_approx(parse("rgba(12%,34%,56%,0.4)").unwrap().rgb(), 31.0, 87.0, 143.0, 0.4);
    assert_rgb_eq(parse("rgba(100%,100%,100%,0.4)").unwrap().rgb(), 255.0, 255.0, 255.0, 0.4);
}

#[test]
fn color_parses_hsl() {
    let c = parse("hsl(60,100%,20%)").unwrap();
    let Color::Hsl(h) = c else { panic!() };
    assert_hsl_eq(h, 60.0, 1.0, 0.2, 1.0);
}

#[test]
fn color_parses_hsla() {
    let c = parse("hsla(60,100%,20%,0.4)").unwrap();
    let Color::Hsl(h) = c else { panic!() };
    assert_hsl_eq(h, 60.0, 1.0, 0.2, 0.4);
}

#[test]
fn color_ignores_surrounding_whitespace() {
    assert_rgb_approx(parse(" aliceblue\t\n").unwrap().rgb(), 240.0, 248.0, 255.0, 1.0);
    assert_rgb_approx(parse(" #abc\t\n").unwrap().rgb(), 170.0, 187.0, 204.0, 1.0);
    assert_rgb_approx(parse(" #aabbcc\t\n").unwrap().rgb(), 170.0, 187.0, 204.0, 1.0);
    assert_rgb_approx(parse(" rgb(120,30,50)\t\n").unwrap().rgb(), 120.0, 30.0, 50.0, 1.0);
    if let Color::Hsl(h) = parse(" hsl(120,30%,50%)\t\n").unwrap() {
        assert_hsl_eq(h, 120.0, 0.3, 0.5, 1.0);
    } else { panic!() }
}

#[test]
fn color_ignores_internal_whitespace() {
    assert_rgb_approx(parse(" rgb( 120 , 30 , 50 ) ").unwrap().rgb(), 120.0, 30.0, 50.0, 1.0);
    assert_rgb_approx(parse(" rgba( 12 , 34 , 56 , 0.4 ) ").unwrap().rgb(), 12.0, 34.0, 56.0, 0.4);
    assert_rgb_approx(parse(" rgba( 12% , 34% , 56% , 0.4 ) ").unwrap().rgb(), 31.0, 87.0, 143.0, 0.4);
    if let Color::Hsl(h) = parse(" hsl( 120 , 30% , 50% ) ").unwrap() {
        assert_hsl_eq(h, 120.0, 0.3, 0.5, 1.0);
    } else { panic!() }
    if let Color::Hsl(h) = parse(" hsla( 60 , 100% , 20% , 0.4 ) ").unwrap() {
        assert_hsl_eq(h, 60.0, 1.0, 0.2, 0.4);
    } else { panic!() }
}

#[test]
fn color_allows_signed_numbers() {
    assert_rgb_approx(parse("rgb(+120,+30,+50)").unwrap().rgb(), 120.0, 30.0, 50.0, 1.0);
    assert_rgb_approx(parse("rgb(-120,-30,-50)").unwrap().rgb(), -120.0, -30.0, -50.0, 1.0);
    if let Color::Hsl(h) = parse("hsl(+120,+30%,+50%)").unwrap() {
        assert_hsl_eq(h, 120.0, 0.3, 0.5, 1.0);
    } else { panic!() }
    if let Color::Hsl(h) = parse("hsl(-120,-30%,-50%)").unwrap() {
        assert_hsl_eq(h, f64::NAN, f64::NAN, -0.5, 1.0);
    } else { panic!() }
    assert_rgb_approx(parse("rgba(12,34,56,+0.4)").unwrap().rgb(), 12.0, 34.0, 56.0, 0.4);
    assert_rgb_approx(parse("rgba(12,34,56,-0.4)").unwrap().rgb(), f64::NAN, f64::NAN, f64::NAN, -0.4);
    assert_rgb_approx(parse("rgba(12%,34%,56%,+0.4)").unwrap().rgb(), 31.0, 87.0, 143.0, 0.4);
    assert_rgb_approx(parse("rgba(12%,34%,56%,-0.4)").unwrap().rgb(), f64::NAN, f64::NAN, f64::NAN, -0.4);
    if let Color::Hsl(h) = parse("hsla(60,100%,20%,+0.4)").unwrap() {
        assert_hsl_eq(h, 60.0, 1.0, 0.2, 0.4);
    } else { panic!() }
    if let Color::Hsl(h) = parse("hsla(60,100%,20%,-0.4)").unwrap() {
        assert_hsl_eq(h, f64::NAN, f64::NAN, f64::NAN, -0.4);
    } else { panic!() }
}

#[test]
fn color_allows_decimals_for_non_integers() {
    assert_rgb_approx(parse("rgb(20.0%,30.4%,51.2%)").unwrap().rgb(), 51.0, 78.0, 131.0, 1.0);
    if let Color::Hsl(h) = parse("hsl(20.0,30.4%,51.2%)").unwrap() {
        assert_hsl_eq(h, 20.0, 0.304, 0.512, 1.0);
    } else { panic!() }
}

#[test]
fn color_allows_leading_decimal() {
    if let Color::Hsl(h) = parse("hsl(.9,.3%,.5%)").unwrap() {
        assert_hsl_eq(h, 0.9, 0.003, 0.005, 1.0);
    } else { panic!() }
    if let Color::Hsl(h) = parse("hsla(.9,.3%,.5%,.5)").unwrap() {
        assert_hsl_eq(h, 0.9, 0.003, 0.005, 0.5);
    } else { panic!() }
    assert_rgb_approx(parse("rgb(.1%,.2%,.3%)").unwrap().rgb(), 0.0, 1.0, 1.0, 1.0);
    assert_rgb_approx(parse("rgba(120,30,50,.5)").unwrap().rgb(), 120.0, 30.0, 50.0, 0.5);
}

#[test]
fn color_allows_exponential() {
    if let Color::Hsl(h) = parse("hsl(1e1,2e1%,3e1%)").unwrap() {
        assert_hsl_eq(h, 10.0, 0.2, 0.3, 1.0);
    } else { panic!() }
    if let Color::Hsl(h) = parse("hsla(9e-1,3e-1%,5e-1%,5e-1)").unwrap() {
        assert_hsl_eq(h, 0.9, 0.003, 0.005, 0.5);
    } else { panic!() }
    assert_rgb_approx(parse("rgb(1e-1%,2e-1%,3e-1%)").unwrap().rgb(), 0.0, 1.0, 1.0, 1.0);
    assert_rgb_approx(parse("rgba(120,30,50,1e-1)").unwrap().rgb(), 120.0, 30.0, 50.0, 0.1);
}

#[test]
fn color_rejects_decimals_for_integers() {
    assert!(parse("rgb(120.5,30,50)").is_none());
}

#[test]
fn color_rejects_empty_decimals() {
    assert!(parse("rgb(120.,30,50)").is_none());
    assert!(parse("rgb(120.%,30%,50%)").is_none());
    assert!(parse("rgba(120,30,50,1.)").is_none());
    assert!(parse("rgba(12%,30%,50%,1.)").is_none());
    assert!(parse("hsla(60,100%,20%,1.)").is_none());
}

#[test]
fn color_rejects_unknown_names() {
    assert!(parse("bostock").is_none());
}

#[test]
fn color_achromatic_alpha_zero() {
    assert_rgb_approx(parse("rgba(0,0,0,0)").unwrap().rgb(), f64::NAN, f64::NAN, f64::NAN, 0.0);
    assert_rgb_approx(parse("#0000").unwrap().rgb(), f64::NAN, f64::NAN, f64::NAN, 0.0);
    assert_rgb_approx(parse("#00000000").unwrap().rgb(), f64::NAN, f64::NAN, f64::NAN, 0.0);
}

#[test]
fn color_rejects_whitespace_around_open_paren() {
    assert!(parse("rgb (120,30,50)").is_none());
    assert!(parse("rgb (12%,30%,50%)").is_none());
    assert!(parse("hsl (120,30%,50%)").is_none());
    assert!(parse("hsl(120,30 %,50%)").is_none());
    assert!(parse("rgba (120,30,50,1)").is_none());
    assert!(parse("rgba (12%,30%,50%,1)").is_none());
    assert!(parse("hsla (120,30%,50%,1)").is_none());
}

#[test]
fn color_is_case_insensitive() {
    assert_rgb_approx(parse("aLiCeBlUE").unwrap().rgb(), 240.0, 248.0, 255.0, 1.0);
    assert_rgb_approx(parse("transPARENT").unwrap().rgb(), f64::NAN, f64::NAN, f64::NAN, 0.0);
    assert_rgb_approx(parse(" #aBc\t\n").unwrap().rgb(), 170.0, 187.0, 204.0, 1.0);
    assert_rgb_approx(parse(" #aaBBCC\t\n").unwrap().rgb(), 170.0, 187.0, 204.0, 1.0);
    assert_rgb_approx(parse(" rGB(120,30,50)\t\n").unwrap().rgb(), 120.0, 30.0, 50.0, 1.0);
    if let Color::Hsl(h) = parse(" HSl(120,30%,50%)\t\n").unwrap() {
        assert_hsl_eq(h, 120.0, 0.3, 0.5, 1.0);
    } else { panic!() }
}

#[test]
fn color_returns_none_for_unknown() {
    assert!(parse("invalid").is_none());
    assert!(parse("hasOwnProperty").is_none());
    assert!(parse("__proto__").is_none());
    assert!(parse("#ab").is_none());
}

#[test]
fn color_format_hex() {
    assert_eq!(parse("rgba(12%,34%,56%,0.4)").unwrap().format_hex(), "#1f578f");
}

// ===========================================================================
// rgb() tests
// ===========================================================================

#[test]
fn rgb_exposes_channels() {
    let c = rgb_from_str("#abc");
    assert_rgb_approx(c, 170.0, 187.0, 204.0, 1.0);
    let c = rgb_from_str("rgba(170, 187, 204, 0.4)");
    assert_rgb_approx(c, 170.0, 187.0, 204.0, 0.4);
}

#[test]
fn rgb_to_string_format() {
    assert_eq!(rgb_from_str("#abcdef").to_string(), "rgb(171, 205, 239)");
    assert_eq!(rgb_from_str("moccasin").to_string(), "rgb(255, 228, 181)");
    assert_eq!(rgb_from_str("hsl(60, 100%, 20%)").to_string(), "rgb(102, 102, 0)");
    assert_eq!(rgb_from_str("rgb(12, 34, 56)").to_string(), "rgb(12, 34, 56)");
    // round-trip through Rgb constructor
    assert_eq!(rgb(12.0, 34.0, 56.0, 1.0).to_string(), "rgb(12, 34, 56)");
    let h = hsl(60.0, 1.0, 0.2, 1.0);
    assert_eq!(h.rgb().to_string(), "rgb(102, 102, 0)");
    assert_eq!(rgb_from_str("rgba(12, 34, 56, 0.4)").to_string(), "rgba(12, 34, 56, 0.4)");
    assert_eq!(rgb_from_str("rgba(12%, 34%, 56%, 0.4)").to_string(), "rgba(31, 87, 143, 0.4)");
    assert_eq!(rgb_from_str("hsla(60, 100%, 20%, 0.4)").to_string(), "rgba(102, 102, 0, 0.4)");
}

#[test]
fn rgb_format_rgb() {
    assert_eq!(rgb_from_str("#abcdef").format_rgb(), "rgb(171, 205, 239)");
    assert_eq!(rgb_from_str("hsl(60, 100%, 20%)").format_rgb(), "rgb(102, 102, 0)");
    assert_eq!(rgb_from_str("rgba(12%, 34%, 56%, 0.4)").format_rgb(), "rgba(31, 87, 143, 0.4)");
    assert_eq!(rgb_from_str("hsla(60, 100%, 20%, 0.4)").format_rgb(), "rgba(102, 102, 0, 0.4)");
}

#[test]
fn rgb_format_hsl() {
    let c = rgb_from_str("#abcdef");
    assert_eq!(Color::Rgb(c).format_hsl(), "hsl(210, 68%, 80.3921568627451%)");
    let c = rgb_from_str("hsl(60, 100%, 20%)");
    assert_eq!(Color::Rgb(c).format_hsl(), "hsl(60, 100%, 20%)");
    let c = rgb_from_str("rgba(12%, 34%, 56%, 0.4)");
    assert_eq!(Color::Rgb(c).format_hsl(), "hsla(210, 64.70588235294117%, 34%, 0.4)");
    let c = rgb_from_str("hsla(60, 100%, 20%, 0.4)");
    assert_eq!(Color::Rgb(c).format_hsl(), "hsla(60, 100%, 20%, 0.4)");
}

#[test]
fn rgb_format_hex() {
    assert_eq!(rgb_from_str("#abcdef").format_hex(), "#abcdef");
    assert_eq!(rgb_from_str("hsl(60, 100%, 20%)").format_hex(), "#666600");
    assert_eq!(rgb_from_str("rgba(12%, 34%, 56%, 0.4)").format_hex(), "#1f578f");
    assert_eq!(rgb_from_str("hsla(60, 100%, 20%, 0.4)").format_hex(), "#666600");
}

#[test]
fn rgb_format_hex8() {
    assert_eq!(rgb_from_str("#abcdef").format_hex8(), "#abcdefff");
    assert_eq!(rgb_from_str("hsl(60, 100%, 20%)").format_hex8(), "#666600ff");
    assert_eq!(rgb_from_str("rgba(12%, 34%, 56%, 0.4)").format_hex8(), "#1f578f66");
    assert_eq!(rgb_from_str("hsla(60, 100%, 20%, 0.4)").format_hex8(), "#66660066");
}

#[test]
fn rgb_to_string_reflects_changes() {
    let mut c = rgb_from_str("#abc");
    c.r += 1.0; c.g += 1.0; c.b += 1.0; c.opacity = 0.5;
    assert_eq!(c.to_string(), "rgba(171, 188, 205, 0.5)");
}

#[test]
fn rgb_to_string_treats_undefined_as_zero() {
    assert_eq!(rgb_from_str("invalid").to_string(), "rgb(0, 0, 0)");
    assert_eq!(rgb(f64::NAN, 12.0, 34.0, 1.0).to_string(), "rgb(0, 12, 34)");
}

#[test]
fn rgb_to_string_undefined_opacity_is_one() {
    let mut c = rgb_from_str("#abc");
    c.r += 1.0; c.g += 1.0; c.b += 1.0; c.opacity = f64::NAN;
    assert_eq!(c.to_string(), "rgb(171, 188, 205)");
}

#[test]
fn rgb_to_string_clamps() {
    assert_eq!(rgb(-1.0,  2.0,  3.0, 1.0).to_string(), "rgb(0, 2, 3)");
    assert_eq!(rgb( 2.0, -1.0,  3.0, 1.0).to_string(), "rgb(2, 0, 3)");
    assert_eq!(rgb( 2.0,  3.0, -1.0, 1.0).to_string(), "rgb(2, 3, 0)");
    assert_eq!(rgb( 2.0,  3.0, -1.0, -0.2).to_string(), "rgba(2, 3, 0, 0)");
    assert_eq!(rgb( 2.0,  3.0, -1.0, 1.2).to_string(), "rgb(2, 3, 0)");
}

#[test]
fn rgb_to_string_rounds() {
    assert_eq!(rgb(0.5, 2.0, 3.0, 1.0).to_string(), "rgb(1, 2, 3)");
    assert_eq!(rgb(2.0, 0.5, 3.0, 1.0).to_string(), "rgb(2, 1, 3)");
    assert_eq!(rgb(2.0, 3.0, 0.5, 1.0).to_string(), "rgb(2, 3, 1)");
}

#[test]
fn rgb_constructor_does_not_round() {
    let c = rgb(1.2, 2.6, 42.9, 1.0);
    assert_rgb_eq(c, 1.2, 2.6, 42.9, 1.0);
}

#[test]
fn rgb_constructor_does_not_clamp() {
    assert_rgb_approx(rgb(-10.0, -20.0, -30.0, 1.0), -10.0, -20.0, -30.0, 1.0);
    assert_rgb_approx(rgb(300.0, 400.0, 500.0, 1.0), 300.0, 400.0, 500.0, 1.0);
}

#[test]
fn rgb_clamp_method() {
    assert_rgb_approx(rgb(-10.0, -20.0, -30.0, 1.0).clamp(), 0.0, 0.0, 0.0, 1.0);
    assert_rgb_approx(rgb(10.5, 20.5, 30.5, 1.0).clamp(), 11.0, 21.0, 31.0, 1.0);
    assert_rgb_approx(rgb(300.0, 400.0, 500.0, 1.0).clamp(), 255.0, 255.0, 255.0, 1.0);
    assert_eq!(rgb(10.5, 20.5, 30.5, -1.0).clamp().opacity, 0.0);
    assert_eq!(rgb(10.5, 20.5, 30.5, 0.5).clamp().opacity, 0.5);
    assert_eq!(rgb(10.5, 20.5, 30.5, 2.0).clamp().opacity, 1.0);
    assert_eq!(rgb(10.5, 20.5, 30.5, f64::NAN).clamp().opacity, 1.0);
}

#[test]
fn rgb_constructor_does_not_clamp_opacity() {
    assert_rgb_approx(rgb(-10.0, -20.0, -30.0, -0.2), -10.0, -20.0, -30.0, -0.2);
    assert_rgb_approx(rgb(300.0, 400.0, 500.0, 1.2), 300.0, 400.0, 500.0, 1.2);
}

#[test]
fn rgb_parse_format() {
    assert_rgb_approx(rgb_from_str("#abcdef"), 171.0, 205.0, 239.0, 1.0);
    assert_rgb_approx(rgb_from_str("#abc"), 170.0, 187.0, 204.0, 1.0);
    assert_rgb_approx(rgb_from_str("rgb(12, 34, 56)"), 12.0, 34.0, 56.0, 1.0);
    assert_rgb_approx(rgb_from_str("rgb(12%, 34%, 56%)"), 31.0, 87.0, 143.0, 1.0);
    assert_rgb_approx(rgb_from_str("hsl(60,100%,20%)"), 102.0, 102.0, 0.0, 1.0);
    assert_rgb_approx(rgb_from_str("aliceblue"), 240.0, 248.0, 255.0, 1.0);
    assert_rgb_approx(rgb_from_str("hsla(60,100%,20%,0.4)"), 102.0, 102.0, 0.0, 0.4);
}

#[test]
fn rgb_parse_alpha_zero() {
    assert_rgb_approx(rgb_from_str("rgba(12,34,45,0)"), f64::NAN, f64::NAN, f64::NAN, 0.0);
    assert_rgb_approx(rgb_from_str("rgba(12,34,45,-0.1)"), f64::NAN, f64::NAN, f64::NAN, -0.1);
}

#[test]
fn rgb_parse_unknown_returns_nan() {
    assert_rgb_approx(rgb_from_str("invalid"), f64::NAN, f64::NAN, f64::NAN, f64::NAN);
}

#[test]
fn rgb_displayable() {
    assert!(rgb_from_str("white").displayable());
    assert!(rgb_from_str("red").displayable());
    assert!(rgb_from_str("black").displayable());
    assert!(!rgb_from_str("invalid").displayable());
    assert!(!rgb(-1.0, 0.0, 0.0, 1.0).displayable());
    assert!(!rgb(0.0, -1.0, 0.0, 1.0).displayable());
    assert!(!rgb(0.0, 0.0, -1.0, 1.0).displayable());
    assert!(!rgb(256.0, 0.0, 0.0, 1.0).displayable());
    assert!(!rgb(0.0, 256.0, 0.0, 1.0).displayable());
    assert!(!rgb(0.0, 0.0, 256.0, 1.0).displayable());
    assert!(rgb(0.0, 0.0, 255.0, 0.0).displayable());
    assert!(!rgb(0.0, 0.0, 255.0, 1.2).displayable());
    assert!(!rgb(0.0, 0.0, 255.0, -0.2).displayable());
}

#[test]
fn rgb_brighter() {
    let c = rgb_from_str("rgba(165, 42, 42, 0.4)");
    assert_rgb_approx(c.brighter(Some(0.5)), 197.0, 50.0, 50.0, 0.4);
    assert_rgb_approx(c.brighter(Some(1.0)), 236.0, 60.0, 60.0, 0.4);
    assert_rgb_approx(c.brighter(Some(2.0)), 337.0, 86.0, 86.0, 0.4);
}

#[test]
fn rgb_brighter_returns_copy() {
    let c1 = rgb_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.brighter(Some(1.0));
    assert_rgb_approx(c1, 70.0, 130.0, 180.0, 0.4);
    assert_rgb_approx(c2, 100.0, 186.0, 257.0, 0.4);
}

#[test]
fn rgb_brighter_default() {
    let c1 = rgb_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.brighter(None);
    let c3 = c1.brighter(Some(1.0));
    assert_rgb_approx(c2, c3.r, c3.g, c3.b, 0.4);
}

#[test]
fn rgb_brighter_negative_is_darker() {
    let c1 = rgb_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.brighter(Some(1.5));
    let c3 = c1.darker(Some(-1.5));
    assert_rgb_approx(c2, c3.r, c3.g, c3.b, 0.4);
}

#[test]
fn rgb_black_brighter_is_black() {
    let c1 = rgb_from_str("black");
    let c2 = c1.brighter(Some(1.0));
    assert_rgb_approx(c1, 0.0, 0.0, 0.0, 1.0);
    assert_rgb_approx(c2, 0.0, 0.0, 0.0, 1.0);
}

#[test]
fn rgb_darker() {
    let c = rgb_from_str("rgba(165, 42, 42, 0.4)");
    assert_rgb_approx(c.darker(Some(0.5)), 138.0, 35.0, 35.0, 0.4);
    assert_rgb_approx(c.darker(Some(1.0)), 115.0, 29.0, 29.0, 0.4);
    assert_rgb_approx(c.darker(Some(2.0)), 81.0, 21.0, 21.0, 0.4);
}

#[test]
fn rgb_rgb_returns_self() {
    let c = rgb(70.0, 130.0, 180.0, 1.0);
    let r2 = c.rgb();
    // Same channel values.
    assert_rgb_eq(r2, c.r, c.g, c.b, c.opacity);
}

// ===========================================================================
// hsl() tests
// ===========================================================================

#[test]
fn hsl_exposes_channels() {
    assert_hsl_eq(hsl_from_str("#abc"), 210.0, 0.25, 0.7333333333333333, 1.0);
    assert_hsl_eq(hsl_from_str("hsla(60, 100%, 20%, 0.4)"), 60.0, 1.0, 0.2, 0.4);
}

#[test]
fn hsl_to_string_converts_to_rgb() {
    assert_eq!(hsl_from_str("#abcdef").to_string(), "rgb(171, 205, 239)");
    assert_eq!(hsl_from_str("moccasin").to_string(), "rgb(255, 228, 181)");
    assert_eq!(hsl_from_str("hsl(60, 100%, 20%)").to_string(), "rgb(102, 102, 0)");
    assert_eq!(hsl_from_str("hsla(60, 100%, 20%, 0.4)").to_string(), "rgba(102, 102, 0, 0.4)");
    assert_eq!(hsl_from_str("rgb(12, 34, 56)").to_string(), "rgb(12, 34, 56)");
    assert_eq!(hsl(60.0, 1.0, 0.2, 1.0).to_string(), "rgb(102, 102, 0)");
    assert_eq!(hsl(60.0, 1.0, 0.2, 0.4).to_string(), "rgba(102, 102, 0, 0.4)");
}

#[test]
fn hsl_format_rgb() {
    assert_eq!(hsl_from_str("#abcdef").rgb().format_rgb(), "rgb(171, 205, 239)");
    assert_eq!(hsl_from_str("hsl(60, 100%, 20%)").rgb().format_rgb(), "rgb(102, 102, 0)");
    assert_eq!(hsl_from_str("rgba(12%, 34%, 56%, 0.4)").rgb().format_rgb(), "rgba(31, 87, 143, 0.4)");
    assert_eq!(hsl_from_str("hsla(60, 100%, 20%, 0.4)").rgb().format_rgb(), "rgba(102, 102, 0, 0.4)");
}

#[test]
fn hsl_format_hsl() {
    assert_eq!(hsl_from_str("#abcdef").format_hsl(), "hsl(210, 68%, 80.3921568627451%)");
    assert_eq!(hsl_from_str("hsl(60, 100%, 20%)").format_hsl(), "hsl(60, 100%, 20%)");
    assert_eq!(hsl_from_str("rgba(12%, 34%, 56%, 0.4)").format_hsl(), "hsla(210, 64.70588235294117%, 34%, 0.4)");
    assert_eq!(hsl_from_str("hsla(60, 100%, 20%, 0.4)").format_hsl(), "hsla(60, 100%, 20%, 0.4)");
}

#[test]
fn hsl_format_hsl_clamps() {
    assert_eq!(hsl(180.0, -100.0, -50.0, 1.0).format_hsl(), "hsl(180, 0%, 0%)");
    assert_eq!(hsl(180.0, 150.0, 200.0, 1.0).format_hsl(), "hsl(180, 100%, 100%)");
    assert_eq!(hsl(-90.0, 50.0, 50.0, 1.0).format_hsl(), "hsl(270, 100%, 100%)");
    assert_eq!(hsl(420.0, 50.0, 50.0, 1.0).format_hsl(), "hsl(60, 100%, 100%)");
}

#[test]
fn hsl_format_hex() {
    assert_eq!(hsl_from_str("#abcdef").rgb().format_hex(), "#abcdef");
    assert_eq!(hsl_from_str("hsl(60, 100%, 20%)").rgb().format_hex(), "#666600");
    assert_eq!(hsl_from_str("rgba(12%, 34%, 56%, 0.4)").rgb().format_hex(), "#1f578f");
    assert_eq!(hsl_from_str("hsla(60, 100%, 20%, 0.4)").rgb().format_hex(), "#666600");
}

#[test]
fn hsl_to_string_reflects_changes() {
    let mut c = hsl_from_str("#abc");
    c.h += 10.0; c.s += 0.01; c.l -= 0.01; c.opacity = 0.4;
    assert_eq!(c.to_string(), "rgba(166, 178, 203, 0.4)");
}

#[test]
fn hsl_to_string_undefined_channels() {
    assert_eq!(hsl_from_str("invalid").to_string(), "rgb(0, 0, 0)");
    assert_eq!(hsl_from_str("#000").to_string(), "rgb(0, 0, 0)");
    assert_eq!(hsl_from_str("#ccc").to_string(), "rgb(204, 204, 204)");
    assert_eq!(hsl_from_str("#fff").to_string(), "rgb(255, 255, 255)");
    assert_eq!(hsl(f64::NAN, 0.5, 0.4, 1.0).to_string(), "rgb(102, 102, 102)");
    assert_eq!(hsl(120.0, f64::NAN, 0.4, 1.0).to_string(), "rgb(102, 102, 102)");
    assert_eq!(hsl(f64::NAN, f64::NAN, 0.4, 1.0).to_string(), "rgb(102, 102, 102)");
    assert_eq!(hsl(120.0, 0.5, f64::NAN, 1.0).to_string(), "rgb(0, 0, 0)");
}

#[test]
fn hsl_to_string_undefined_opacity_is_one() {
    let mut c = hsl_from_str("#abc");
    c.opacity = f64::NAN;
    assert_eq!(c.to_string(), "rgb(170, 187, 204)");
}

#[test]
fn hsl_constructor_does_not_wrap_hue() {
    assert_hsl_eq(hsl(-10.0, 0.4, 0.5, 1.0), -10.0, 0.4, 0.5, 1.0);
    assert_hsl_eq(hsl(0.0, 0.4, 0.5, 1.0), 0.0, 0.4, 0.5, 1.0);
    assert_hsl_eq(hsl(360.0, 0.4, 0.5, 1.0), 360.0, 0.4, 0.5, 1.0);
    assert_hsl_eq(hsl(370.0, 0.4, 0.5, 1.0), 370.0, 0.4, 0.5, 1.0);
}

#[test]
fn hsl_constructor_does_not_clamp_sl() {
    assert_hsl_eq(hsl(120.0, -0.1, 0.5, 1.0), 120.0, -0.1, 0.5, 1.0);
    assert_hsl_eq(hsl(120.0, 1.1, 0.5, 1.0), 120.0, 1.1, 0.5, 1.0);
    assert_hsl_eq(hsl(120.0, 0.2, -0.1, 1.0), 120.0, 0.2, -0.1, 1.0);
    assert_hsl_eq(hsl(120.0, 0.2, 1.1, 1.0), 120.0, 0.2, 1.1, 1.0);
}

#[test]
fn hsl_clamp_method() {
    assert_hsl_eq(hsl(120.0, -0.1, -0.2, 1.0).clamp(), 120.0, 0.0, 0.0, 1.0);
    assert_hsl_eq(hsl(120.0, 1.1, 1.2, 1.0).clamp(), 120.0, 1.0, 1.0, 1.0);
    assert_hsl_eq(hsl(120.0, 2.1, 2.2, 1.0).clamp(), 120.0, 1.0, 1.0, 1.0);
    assert_hsl_eq(hsl(420.0, -0.1, -0.2, 1.0).clamp(), 60.0, 0.0, 0.0, 1.0);
    assert_hsl_eq(hsl(-420.0, -0.1, -0.2, 1.0).clamp(), 300.0, 0.0, 0.0, 1.0);
    assert_eq!(hsl(-420.0, -0.1, -0.2, f64::NAN).clamp().opacity, 1.0);
    assert_eq!(hsl(-420.0, -0.1, -0.2, 0.5).clamp().opacity, 0.5);
    assert_eq!(hsl(-420.0, -0.1, -0.2, -1.0).clamp().opacity, 0.0);
    assert_eq!(hsl(-420.0, -0.1, -0.2, 2.0).clamp().opacity, 1.0);
}

#[test]
fn hsl_preserves_explicit_hue_for_grays() {
    assert_hsl_eq(hsl(0.0, 0.0, 0.0, 1.0), 0.0, 0.0, 0.0, 1.0);
    assert_hsl_eq(hsl(42.0, 0.0, 0.5, 1.0), 42.0, 0.0, 0.5, 1.0);
    assert_hsl_eq(hsl(118.0, 0.0, 1.0, 1.0), 118.0, 0.0, 1.0, 1.0);
}

#[test]
fn hsl_parse_format() {
    assert_hsl_eq(hsl_from_str("#abcdef"), 210.0, 0.68, 0.803921568627451, 1.0);
    assert_hsl_eq(hsl_from_str("#abc"), 210.0, 0.25, 0.7333333333333333, 1.0);
    assert_hsl_eq(hsl_from_str("rgb(12, 34, 56)"), 210.0, 0.6470588235294118, 0.13333333333333333, 1.0);
    assert_hsl_eq(hsl_from_str("rgb(12%, 34%, 56%)"), 210.0, 0.6470588235294117, 0.34, 1.0);
    assert_hsl_eq(hsl_from_str("hsl(60,100%,20%)"), 60.0, 1.0, 0.2, 1.0);
    assert_hsl_eq(hsl_from_str("hsla(60,100%,20%,0.4)"), 60.0, 1.0, 0.2, 0.4);
    assert_hsl_eq(hsl_from_str("aliceblue"), 208.0, 1.0, 0.9705882352941176, 1.0);
    assert_hsl_eq(hsl_from_str("transparent"), f64::NAN, f64::NAN, f64::NAN, 0.0);
}

#[test]
fn hsl_parse_ignores_hue_if_saturation_zero() {
    assert_hsl_eq(hsl_from_str("hsl(120,0%,20%)"), f64::NAN, 0.0, 0.2, 1.0);
    assert_hsl_eq(hsl_from_str("hsl(120,-10%,20%)"), f64::NAN, -0.1, 0.2, 1.0);
}

#[test]
fn hsl_parse_ignores_hs_when_l_extreme() {
    assert_hsl_eq(hsl_from_str("hsl(120,20%,-10%)"), f64::NAN, f64::NAN, -0.1, 1.0);
    assert_hsl_eq(hsl_from_str("hsl(120,20%,0%)"),  f64::NAN, f64::NAN, 0.0, 1.0);
    assert_hsl_eq(hsl_from_str("hsl(120,20%,100%)"), f64::NAN, f64::NAN, 1.0, 1.0);
    assert_hsl_eq(hsl_from_str("hsl(120,20%,120%)"), f64::NAN, f64::NAN, 1.2, 1.0);
}

#[test]
fn hsl_parse_alpha_zero() {
    assert_hsl_eq(hsl_from_str("hsla(120,20%,10%,0)"),    f64::NAN, f64::NAN, f64::NAN, 0.0);
    assert_hsl_eq(hsl_from_str("hsla(120,20%,10%,-0.1)"), f64::NAN, f64::NAN, f64::NAN, -0.1);
}

#[test]
fn hsl_parse_no_precision_loss() {
    assert_hsl_eq(hsl_from_str("hsl(325,50%,40%)"), 325.0, 0.5, 0.4, 1.0);
}

#[test]
fn hsl_unknown_returns_nan() {
    assert_hsl_eq(hsl_from_str("invalid"), f64::NAN, f64::NAN, f64::NAN, f64::NAN);
}

#[test]
fn hsl_from_rgb() {
    assert_hsl_eq(hsl_from(&Color::Rgb(rgb(255.0, 0.0, 0.0, 0.4))), 0.0, 1.0, 0.5, 0.4);
}

#[test]
fn hsl_grays_have_undefined_hue() {
    assert_hsl_eq(hsl_from_str("gray"), f64::NAN, 0.0, 0.5019607843137255, 1.0);
    assert_hsl_eq(hsl_from_str("#ccc"), f64::NAN, 0.0, 0.8, 1.0);
}

#[test]
fn hsl_black_white_undefined_hs() {
    assert_hsl_eq(hsl_from_str("black"), f64::NAN, f64::NAN, 0.0, 1.0);
    assert_hsl_eq(hsl_from_str("#000"),  f64::NAN, f64::NAN, 0.0, 1.0);
    assert_hsl_eq(hsl_from_str("white"), f64::NAN, f64::NAN, 1.0, 1.0);
    assert_hsl_eq(hsl_from_str("#fff"),  f64::NAN, f64::NAN, 1.0, 1.0);
}

#[test]
fn hsl_displayable() {
    assert!(hsl_from_str("white").displayable());
    assert!(hsl_from_str("red").displayable());
    assert!(hsl_from_str("black").displayable());
    assert!(!hsl_from_str("invalid").displayable());
    assert!(hsl(f64::NAN, f64::NAN, 1.0, 1.0).displayable());
    assert!(!hsl(f64::NAN, f64::NAN, 1.5, 1.0).displayable());
    assert!(!hsl(120.0, -0.5, 0.0, 1.0).displayable());
    assert!(!hsl(120.0, 1.5, 0.0, 1.0).displayable());
    assert!(hsl(0.0, 1.0, 1.0, 0.0).displayable());
    assert!(hsl(0.0, 1.0, 1.0, 1.0).displayable());
    assert!(!hsl(0.0, 1.0, 1.0, -0.2).displayable());
    assert!(!hsl(0.0, 1.0, 1.0, 1.2).displayable());
}

#[test]
fn hsl_brighter() {
    let c = hsl_from_str("rgba(165, 42, 42, 0.4)");
    let b1 = c.brighter(Some(0.5));
    assert_hsl_eq(b1, 0.0, 0.5942028985507246, 0.48512221735624066, 0.4);
    let b2 = c.brighter(Some(1.0));
    assert_hsl_eq(b2, 0.0, 0.5942028985507246, 0.5798319327731092, 0.4);
}

#[test]
fn hsl_brighter_returns_copy() {
    let c1 = hsl_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.brighter(Some(1.0));
    assert_hsl_eq(c1, 207.27272727272728, 0.44, 0.49019607843137253, 0.4);
    assert_hsl_eq(c2, 207.27272727272728, 0.44, 0.7002801120448179, 0.4);
}

#[test]
fn hsl_brighter_default() {
    let c1 = hsl_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.brighter(None);
    let c3 = c1.brighter(Some(1.0));
    assert_hsl_eq(c2, c3.h, c3.s, c3.l, 0.4);
}

#[test]
fn hsl_black_brighter_is_black() {
    let c1 = hsl_from_str("black");
    let c2 = c1.brighter(Some(1.0));
    assert_hsl_eq(c1, f64::NAN, f64::NAN, 0.0, 1.0);
    assert_hsl_eq(c2, f64::NAN, f64::NAN, 0.0, 1.0);
}

#[test]
fn hsl_darker_returns_copy() {
    let c1 = hsl_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.darker(Some(1.0));
    assert_hsl_eq(c1, 207.27272727272728, 0.44, 0.49019607843137253, 0.4);
    assert_hsl_eq(c2, 207.27272727272728, 0.44, 0.34313725490196073, 0.4);
}

#[test]
fn hsl_to_rgb() {
    let c = hsl(120.0, 0.3, 0.5, 0.4);
    assert_rgb_approx(c.rgb(), 89.0, 166.0, 89.0, 0.4);
}

// ===========================================================================
// lab() tests
// ===========================================================================

#[test]
fn lab_exposes_channels() {
    assert_lab_eq(
        lab_from_str("rgba(170, 187, 204, 0.4)"),
        74.96879980931759, -3.398998724348956, -10.696507207853333, 0.4,
    );
}

#[test]
fn lab_to_string_converts_to_rgb() {
    assert_eq!(lab_from_str("#abcdef").to_string(), "rgb(171, 205, 239)");
    assert_eq!(lab_from_str("moccasin").to_string(), "rgb(255, 228, 181)");
    assert_eq!(lab_from_str("hsl(60, 100%, 20%)").to_string(), "rgb(102, 102, 0)");
    assert_eq!(lab_from_str("hsla(60, 100%, 20%, 0.4)").to_string(), "rgba(102, 102, 0, 0.4)");
    assert_eq!(lab_from_str("rgb(12, 34, 56)").to_string(), "rgb(12, 34, 56)");
}

#[test]
fn lab_constructor_does_not_clamp() {
    assert_lab_eq(lab(-10.0, 1.0, 2.0, 1.0), -10.0, 1.0, 2.0, 1.0);
    assert_lab_eq(lab(110.0, 1.0, 2.0, 1.0), 110.0, 1.0, 2.0, 1.0);
}

#[test]
fn lab_undefined_channels_become_zero_in_rgb() {
    assert_eq!(lab_from_str("invalid").to_string(), "rgb(0, 0, 0)");
    assert_eq!(lab(f64::NAN, 0.0, 0.0, 1.0).to_string(), "rgb(0, 0, 0)");
    assert_eq!(lab(50.0, f64::NAN, 0.0, 1.0).to_string(), "rgb(119, 119, 119)");
    assert_eq!(lab(50.0, 0.0, f64::NAN, 1.0).to_string(), "rgb(119, 119, 119)");
    assert_eq!(lab(50.0, f64::NAN, f64::NAN, 1.0).to_string(), "rgb(119, 119, 119)");
}

#[test]
fn lab_parse_format() {
    assert_lab_eq(lab_from_str("#abcdef"),
        80.77135418262527, -5.957098328496224, -20.785782794739237, 1.0);
    assert_lab_eq(lab_from_str("#abc"),
        74.96879980931759, -3.398998724348956, -10.696507207853333, 1.0);
    assert_lab_eq(lab_from_str("rgb(12, 34, 56)"),
        12.404844123471648, -2.159950219712034, -17.168132391132946, 1.0);
    assert_lab_eq(lab_from_str("rgb(12%, 34%, 56%)"),
        35.48300043476593, -2.507637675606522, -36.95112983195855, 1.0);
    assert_lab_eq(lab_from_str("rgba(12%, 34%, 56%, 0.4)"),
        35.48300043476593, -2.507637675606522, -36.95112983195855, 0.4);
    assert_lab_eq(lab_from_str("hsl(60,100%,20%)"),
        41.97125732118659, -8.03835128380484, 47.65411917854332, 1.0);
    assert_lab_eq(lab_from_str("hsla(60,100%,20%,0.4)"),
        41.97125732118659, -8.03835128380484, 47.65411917854332, 0.4);
    assert_lab_eq(lab_from_str("aliceblue"),
        97.12294991108756, -1.773836604137824, -4.332680308569969, 1.0);
}

#[test]
fn lab_unknown_returns_nan() {
    assert_lab_eq(lab_from_str("invalid"), f64::NAN, f64::NAN, f64::NAN, f64::NAN);
}

#[test]
fn lab_hcl_lab_preserves_a_b_when_l_is_zero() {
    let l = lab(0.0, 10.0, 0.0, 1.0);
    let h = hcl_from(&Color::Lab(l));
    let l2 = lab_from(&Color::Hcl(h));
    assert_lab_eq(l2, 0.0, 10.0, 0.0, 1.0);
}

#[test]
fn lab_brighter() {
    let c = lab_from_str("rgba(165, 42, 42, 0.4)");
    let b1 = c.brighter(Some(0.5));
    assert_lab_eq(b1, 47.149667346714935, 50.388769337115, 31.834059255569358, 0.4);
    let b2 = c.brighter(Some(1.0));
    assert_lab_eq(b2, 56.149667346714935, 50.388769337115, 31.834059255569358, 0.4);
    let b3 = c.brighter(Some(2.0));
    assert_lab_eq(b3, 74.14966734671493, 50.388769337115, 31.834059255569358, 0.4);
}

#[test]
fn lab_darker() {
    let c = lab_from_str("rgba(165, 42, 42, 0.4)");
    assert_lab_eq(c.darker(Some(0.5)), 29.149667346714935, 50.388769337115, 31.834059255569358, 0.4);
    assert_lab_eq(c.darker(Some(1.0)), 20.149667346714935, 50.388769337115, 31.834059255569358, 0.4);
    assert_lab_eq(c.darker(Some(2.0)),  2.149667346714935, 50.388769337115, 31.834059255569358, 0.4);
}

#[test]
fn lab_to_rgb() {
    let c = lab(50.0, 4.0, -5.0, 0.4);
    assert_rgb_approx(c.rgb(), 123.0, 117.0, 128.0, 0.4);
}

#[test]
fn gray_is_alias_for_lab() {
    assert_lab_eq(gray(120.0, None), 120.0, 0.0, 0.0, 1.0);
    assert_lab_eq(gray(120.0, Some(0.5)), 120.0, 0.0, 0.0, 0.5);
}

// ===========================================================================
// hcl() tests
// ===========================================================================

#[test]
fn hcl_exposes_channels() {
    assert_hcl_eq(hcl_from_str("#abc"),
        252.37145234745182, 11.223567114593477, 74.96879980931759, 1.0);
}

#[test]
fn hcl_black_white_undefined() {
    assert_hcl_eq(hcl_from_str("black"), f64::NAN, f64::NAN, 0.0, 1.0);
    assert_hcl_eq(hcl_from_str("#000"),  f64::NAN, f64::NAN, 0.0, 1.0);
    assert_hcl_eq(hcl_from_str("white"), f64::NAN, f64::NAN, 100.0, 1.0);
    assert_hcl_eq(hcl_from_str("#fff"),  f64::NAN, f64::NAN, 100.0, 1.0);
}

#[test]
fn hcl_gray_undefined_h_zero_c() {
    assert_hcl_eq(hcl_from_str("gray"), f64::NAN, 0.0, 53.585013452169036, 1.0);
}

#[test]
fn hcl_to_string() {
    assert_eq!(hcl_from_str("#abcdef").to_string(), "rgb(171, 205, 239)");
    assert_eq!(hcl_from_str("moccasin").to_string(), "rgb(255, 228, 181)");
    assert_eq!(hcl_from_str("hsl(60, 100%, 20%)").to_string(), "rgb(102, 102, 0)");
    assert_eq!(hcl_from_str("rgb(12, 34, 56)").to_string(), "rgb(12, 34, 56)");
}

#[test]
fn hcl_undefined_channels() {
    assert_eq!(hcl_from_str("invalid").to_string(), "rgb(0, 0, 0)");
    assert_eq!(hcl(f64::NAN, 20.0, 40.0, 1.0).to_string(), "rgb(94, 94, 94)");
    assert_eq!(hcl(120.0, f64::NAN, 40.0, 1.0).to_string(), "rgb(94, 94, 94)");
    assert_eq!(hcl(0.0, f64::NAN, 40.0, 1.0).to_string(), "rgb(94, 94, 94)");
    assert_eq!(hcl(120.0, 50.0, f64::NAN, 1.0).to_string(), "rgb(0, 0, 0)");
    assert_eq!(hcl(0.0, 50.0, f64::NAN, 1.0).to_string(), "rgb(0, 0, 0)");
    assert_eq!(hcl(120.0, 0.0, f64::NAN, 1.0).to_string(), "rgb(0, 0, 0)");
}

#[test]
fn hcl_yellow_displayable() {
    assert!(hcl_from_str("yellow").displayable());
    assert_eq!(hcl_from_str("yellow").to_string(), "rgb(255, 255, 0)");
}

#[test]
fn hcl_does_not_wrap_hue() {
    assert_hcl_eq(hcl(-10.0, 40.0, 50.0, 1.0), -10.0, 40.0, 50.0, 1.0);
    assert_hcl_eq(hcl(360.0, 40.0, 50.0, 1.0), 360.0, 40.0, 50.0, 1.0);
    assert_hcl_eq(hcl(370.0, 40.0, 50.0, 1.0), 370.0, 40.0, 50.0, 1.0);
}

#[test]
fn hcl_parse_format() {
    assert_hcl_eq(hcl_from_str("#abcdef"),
        254.0079700170605, 21.62257586147983, 80.77135418262527, 1.0);
    assert_hcl_eq(hcl_from_str("#abc"),
        252.37145234745182, 11.223567114593477, 74.96879980931759, 1.0);
    assert_hcl_eq(hcl_from_str("rgb(12, 34, 56)"),
        262.8292023352897, 17.30347233219686, 12.404844123471648, 1.0);
    assert_hcl_eq(hcl_from_str("rgb(12%, 34%, 56%)"),
        266.117653326772, 37.03612078188506, 35.48300043476593, 1.0);
    assert_hcl_eq(hcl_from_str("rgba(12%, 34%, 56%, 0.4)"),
        266.117653326772, 37.03612078188506, 35.48300043476593, 0.4);
    assert_hcl_eq(hcl_from_str("hsl(60,100%,20%)"),
        99.57458688693686, 48.327323183108916, 41.97125732118659, 1.0);
    assert_hcl_eq(hcl_from_str("aliceblue"),
        247.7353849904697, 4.681732046417135, 97.12294991108756, 1.0);
}

#[test]
fn hcl_unknown_returns_nan() {
    assert_hcl_eq(hcl_from_str("invalid"), f64::NAN, f64::NAN, f64::NAN, f64::NAN);
}

#[test]
fn hcl_from_lab_zero_chroma() {
    assert_hcl_eq(hcl_from(&Color::Lab(lab(0.0, 0.0, 0.0, 1.0))),   f64::NAN, f64::NAN, 0.0,   1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(50.0, 0.0, 0.0, 1.0))),  f64::NAN, 0.0,      50.0,  1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(100.0, 0.0, 0.0, 1.0))), f64::NAN, f64::NAN, 100.0, 1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(0.0, 10.0, 0.0, 1.0))),  0.0, 10.0, 0.0, 1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(50.0, 10.0, 0.0, 1.0))), 0.0, 10.0, 50.0, 1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(100.0, 10.0, 0.0, 1.0))), 0.0, 10.0, 100.0, 1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(0.0, 0.0, 10.0, 1.0))),  90.0, 10.0, 0.0, 1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(50.0, 0.0, 10.0, 1.0))), 90.0, 10.0, 50.0, 1.0);
    assert_hcl_eq(hcl_from(&Color::Lab(lab(100.0, 0.0, 10.0, 1.0))), 90.0, 10.0, 100.0, 1.0);
}

#[test]
fn hcl_brighter_returns_copy() {
    let c1 = hcl_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.brighter(Some(1.0));
    assert_hcl_eq(c1, 255.71009124439382, 33.88100417355615, 51.98624890550498, 0.4);
    assert_hcl_eq(c2, 255.71009124439382, 33.88100417355615, 69.98624890550498, 0.4);
}

#[test]
fn hcl_darker_returns_copy() {
    let c1 = hcl_from_str("rgba(70, 130, 180, 0.4)");
    let c2 = c1.darker(Some(1.0));
    assert_hcl_eq(c1, 255.71009124439382, 33.88100417355615, 51.98624890550498, 0.4);
    assert_hcl_eq(c2, 255.71009124439382, 33.88100417355615, 33.98624890550498, 0.4);
}

#[test]
fn hcl_to_rgb() {
    let c = hcl(120.0, 30.0, 50.0, 0.4);
    assert_rgb_approx(c.rgb(), 105.0, 126.0, 73.0, 0.4);
}

// ===========================================================================
// lch() tests
// ===========================================================================

#[test]
fn lch_alias_for_hcl() {
    assert_hcl_eq(lch(74.0, 11.0, 252.0, None), 252.0, 11.0, 74.0, 1.0);
    assert_hcl_eq(lch(74.0, 11.0, 252.0, Some(0.5)), 252.0, 11.0, 74.0, 0.5);
}

// ===========================================================================
// cubehelix() tests
// ===========================================================================

#[test]
fn cubehelix_basic() {
    let c = cubehelix_from_str("steelblue");
    // No specific numeric expectation from upstream tests beyond construction.
    // Validate round-trip back to RGB ≈ steelblue (70, 130, 180).
    assert_rgb_approx(c.rgb(), 70.0, 130.0, 180.0, 1.0);
}
