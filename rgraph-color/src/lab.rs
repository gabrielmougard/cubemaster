//! CIE Lab and HCL color spaces (D50 white point), plus the `gray(l)` and
//! `lch(l, c, h)` helpers from d3-color.
//!
//! Reference: <https://observablehq.com/@mbostock/lab-and-rgb>.

use core::fmt;

use crate::color::{Color, ColorSpace};
use crate::rgb::Rgb;

const K: f64 = 18.0;
const XN: f64 = 0.96422;
const YN: f64 = 1.0;
const ZN: f64 = 0.82521;
const T0: f64 = 4.0 / 29.0;
const T1: f64 = 6.0 / 29.0;
// T2 = 3 * T1^2, T3 = T1^3 — kept as functions to avoid `const_fn_floating_point_arithmetic`
// instability.
fn t2() -> f64 { 3.0 * T1 * T1 }
fn t3() -> f64 { T1 * T1 * T1 }

const RADIANS: f64 = core::f64::consts::PI / 180.0;
const DEGREES: f64 = 180.0 / core::f64::consts::PI;

// ---------------------------------------------------------------------------
// Lab
// ---------------------------------------------------------------------------

/// CIE Lab color (D50 white point). `l` ranges roughly `[0, 100]`, `a` and
/// `b` are unbounded. NaN channels are honored.
#[derive(Clone, Copy, Debug)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
    pub opacity: f64,
}

impl Lab {
    pub const fn new(l: f64, a: f64, b: f64, opacity: f64) -> Self {
        Lab { l, a, b, opacity }
    }

    pub fn empty() -> Self {
        Lab { l: f64::NAN, a: f64::NAN, b: f64::NAN, opacity: f64::NAN }
    }

    /// `true` when the color projects into the displayable RGB gamut.
    pub fn displayable(&self) -> bool {
        self.rgb().displayable()
    }
}

impl ColorSpace for Lab {
    fn rgb(&self) -> Rgb {
        let y = (self.l + 16.0) / 116.0;
        let x = if self.a.is_nan() { y } else { y + self.a / 500.0 };
        let z = if self.b.is_nan() { y } else { y - self.b / 200.0 };
        let x = XN * lab2xyz(x);
        let y = YN * lab2xyz(y);
        let z = ZN * lab2xyz(z);
        Rgb::new(
            lrgb2rgb( 3.1338561 * x - 1.6168667 * y - 0.4906146 * z),
            lrgb2rgb(-0.9787684 * x + 1.9161415 * y + 0.0334540 * z),
            lrgb2rgb( 0.0719453 * x - 0.2289914 * y + 1.4052427 * z),
            self.opacity,
        )
    }

    fn brighter(&self, k: Option<f64>) -> Self {
        let k = k.unwrap_or(1.0);
        Lab::new(self.l + K * k, self.a, self.b, self.opacity)
    }

    fn darker(&self, k: Option<f64>) -> Self {
        let k = k.unwrap_or(1.0);
        Lab::new(self.l - K * k, self.a, self.b, self.opacity)
    }

    fn into_color(self) -> Color {
        Color::Lab(self)
    }
}

impl fmt::Display for Lab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.rgb().format_rgb())
    }
}

/// Convert any color into Lab. Mirrors d3's `labConvert`.
pub fn from_color(c: &Color) -> Lab {
    match c {
        Color::Lab(lab) => *lab,
        Color::Hcl(hcl) => hcl2lab(hcl),
        _ => rgb_to_lab(&c.rgb()),
    }
}

fn rgb_to_lab(o: &Rgb) -> Lab {
    let r = rgb2lrgb(o.r);
    let g = rgb2lrgb(o.g);
    let b = rgb2lrgb(o.b);
    let y = xyz2lab((0.2225045 * r + 0.7168786 * g + 0.0606169 * b) / YN);
    let (x, z) = if r == g && g == b {
        (y, y)
    } else {
        (
            xyz2lab((0.4360747 * r + 0.3850649 * g + 0.1430804 * b) / XN),
            xyz2lab((0.0139322 * r + 0.0971045 * g + 0.7141733 * b) / ZN),
        )
    };
    Lab::new(116.0 * y - 16.0, 500.0 * (x - y), 200.0 * (y - z), o.opacity)
}

/// `gray(l, opacity)` — a shorthand for `Lab { l, a: 0, b: 0, opacity }`.
pub fn gray(l: f64, opacity: Option<f64>) -> Lab {
    Lab::new(l, 0.0, 0.0, opacity.unwrap_or(1.0))
}

fn xyz2lab(t: f64) -> f64 {
    if t > t3() { t.cbrt() } else { t / t2() + T0 }
}

fn lab2xyz(t: f64) -> f64 {
    if t > T1 { t * t * t } else { t2() * (t - T0) }
}

fn lrgb2rgb(x: f64) -> f64 {
    255.0 * if x <= 0.0031308 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 }
}

fn rgb2lrgb(x: f64) -> f64 {
    let x = x / 255.0;
    if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
}

// ---------------------------------------------------------------------------
// HCL
// ---------------------------------------------------------------------------

/// CIE HCL — Lab in polar coordinates. `h` in degrees, `c` is chroma.
#[derive(Clone, Copy, Debug)]
pub struct Hcl {
    pub h: f64,
    pub c: f64,
    pub l: f64,
    pub opacity: f64,
}

impl Hcl {
    pub const fn new(h: f64, c: f64, l: f64, opacity: f64) -> Self {
        Hcl { h, c, l, opacity }
    }

    pub fn empty() -> Self {
        Hcl { h: f64::NAN, c: f64::NAN, l: f64::NAN, opacity: f64::NAN }
    }

    /// `true` when the color projects into the displayable RGB gamut.
    pub fn displayable(&self) -> bool {
        self.rgb().displayable()
    }
}

impl ColorSpace for Hcl {
    fn rgb(&self) -> Rgb {
        hcl2lab(self).rgb()
    }

    fn brighter(&self, k: Option<f64>) -> Self {
        let k = k.unwrap_or(1.0);
        Hcl::new(self.h, self.c, self.l + K * k, self.opacity)
    }

    fn darker(&self, k: Option<f64>) -> Self {
        let k = k.unwrap_or(1.0);
        Hcl::new(self.h, self.c, self.l - K * k, self.opacity)
    }

    fn into_color(self) -> Color {
        Color::Hcl(self)
    }
}

impl fmt::Display for Hcl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.rgb().format_rgb())
    }
}

/// Convert any color into HCL. Mirrors d3's `hclConvert`.
pub fn hcl_from_color(c: &Color) -> Hcl {
    if let Color::Hcl(h) = c {
        return *h;
    }
    let o = if let Color::Lab(l) = c { *l } else { from_color(c) };
    if o.a == 0.0 && o.b == 0.0 {
        return Hcl::new(
            f64::NAN,
            if o.l > 0.0 && o.l < 100.0 { 0.0 } else { f64::NAN },
            o.l,
            o.opacity,
        );
    }
    let h = o.b.atan2(o.a) * DEGREES;
    Hcl::new(
        if h < 0.0 { h + 360.0 } else { h },
        (o.a * o.a + o.b * o.b).sqrt(),
        o.l,
        o.opacity,
    )
}

fn hcl2lab(o: &Hcl) -> Lab {
    if o.h.is_nan() {
        return Lab::new(o.l, 0.0, 0.0, o.opacity);
    }
    let h = o.h * RADIANS;
    Lab::new(o.l, h.cos() * o.c, h.sin() * o.c, o.opacity)
}

/// `lch(l, c, h, opacity)` — re-orders args to construct an HCL.
pub fn lch(l: f64, c: f64, h: f64, opacity: Option<f64>) -> Hcl {
    Hcl::new(h, c, l, opacity.unwrap_or(1.0))
}
