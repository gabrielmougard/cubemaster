//! HSL color space.

use core::fmt;

use crate::color::{BRIGHTER, Color, ColorSpace, DARKER};
use crate::rgb::{Rgb, clampa, fmt_num};

/// HSL color. Hue in degrees (0..360 conceptually but unbounded), saturation
/// and lightness in `[0, 1]` (also unbounded). NaN means "undefined" — d3
/// uses NaN hue for grays, and NaN h+s for pure black/white.
#[derive(Clone, Copy, Debug)]
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
    pub opacity: f64,
}

impl Hsl {
    pub const fn new(h: f64, s: f64, l: f64, opacity: f64) -> Self {
        Hsl { h, s, l, opacity }
    }

    pub fn empty() -> Self {
        Hsl { h: f64::NAN, s: f64::NAN, l: f64::NAN, opacity: f64::NAN }
    }

    /// Returns a copy with hue wrapped to `[0,360)` and s/l clamped to
    /// `[0,1]`. NaN h or s become 0. Matches `hsl.clamp()` in d3.
    pub fn clamp(&self) -> Self {
        Hsl {
            h: clamph(self.h),
            s: clampt(self.s),
            l: clampt(self.l),
            opacity: clampa(self.opacity),
        }
    }

    /// `hsl(...)` / `hsla(...)` formatting, matching d3 exactly: percent
    /// values are emitted without a fixed number of decimals (whatever the
    /// shortest round-trip representation is).
    pub fn format_hsl(&self) -> String {
        let a = clampa(self.opacity);
        let s = clampt(self.s) * 100.0;
        let l = clampt(self.l) * 100.0;
        if (a - 1.0).abs() < f64::EPSILON {
            format!("hsl({}, {}%, {}%)", clamph(self.h), fmt_num(s), fmt_num(l))
        } else {
            format!(
                "hsla({}, {}%, {}%, {})",
                clamph(self.h),
                fmt_num(s),
                fmt_num(l),
                fmt_num(a)
            )
        }
    }

    /// `true` when components are within their natural ranges.
    pub fn displayable(&self) -> bool {
        ((0.0..=1.0).contains(&self.s) || self.s.is_nan())
            && (0.0..=1.0).contains(&self.l)
            && (0.0..=1.0).contains(&self.opacity)
    }
}

impl ColorSpace for Hsl {
    fn rgb(&self) -> Rgb {
        // Wrap hue into [0,360). NaN h becomes 0 with s forced to 0.
        let mut h = self.h;
        if !h.is_nan() {
            h = h.rem_euclid(360.0);
        }
        let s = if h.is_nan() || self.s.is_nan() { 0.0 } else { self.s };
        let h = if h.is_nan() { 0.0 } else { h };
        let l = self.l;
        let m2 = l + (if l < 0.5 { l } else { 1.0 - l }) * s;
        let m1 = 2.0 * l - m2;
        Rgb::new(
            hsl2rgb(if h >= 240.0 { h - 240.0 } else { h + 120.0 }, m1, m2),
            hsl2rgb(h, m1, m2),
            hsl2rgb(if h < 120.0 { h + 240.0 } else { h - 120.0 }, m1, m2),
            self.opacity,
        )
    }

    fn brighter(&self, k: Option<f64>) -> Self {
        let k = match k {
            None => BRIGHTER,
            Some(v) => BRIGHTER.powf(v),
        };
        Hsl::new(self.h, self.s, self.l * k, self.opacity)
    }

    fn darker(&self, k: Option<f64>) -> Self {
        let k = match k {
            None => DARKER,
            Some(v) => DARKER.powf(v),
        };
        Hsl::new(self.h, self.s, self.l * k, self.opacity)
    }

    fn into_color(self) -> Color {
        Color::Hsl(self)
    }
}

impl fmt::Display for Hsl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // d3's `toString` for hsl converts to RGB first.
        f.write_str(&self.rgb().format_rgb())
    }
}

// ---------------------------------------------------------------------------
// Conversions and helpers
// ---------------------------------------------------------------------------

/// Convert any color into HSL, replicating d3's `hslConvert` carefully so
/// the NaN behaviour (undefined hue for grays) is preserved.
pub fn from_color(c: &Color) -> Hsl {
    if let Color::Hsl(h) = c {
        return *h;
    }
    let o = c.rgb();
    let r = o.r / 255.0;
    let g = o.g / 255.0;
    let b = o.b / 255.0;
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let mut h = f64::NAN;
    let mut s = max - min;
    let l = (max + min) / 2.0;
    if s != 0.0 {
        if r == max {
            h = (g - b) / s + if g < b { 6.0 } else { 0.0 };
        } else if g == max {
            h = (b - r) / s + 2.0;
        } else {
            h = (r - g) / s + 4.0;
        }
        s /= if l < 0.5 { max + min } else { 2.0 - max - min };
        h *= 60.0;
    } else {
        s = if l > 0.0 && l < 1.0 { 0.0 } else { f64::NAN };
    }
    Hsl::new(h, s, l, o.opacity)
}

/// From FvD 13.37, CSS Color Module Level 3.
fn hsl2rgb(h: f64, m1: f64, m2: f64) -> f64 {
    (if h < 60.0 {
        m1 + (m2 - m1) * h / 60.0
    } else if h < 180.0 {
        m2
    } else if h < 240.0 {
        m1 + (m2 - m1) * (240.0 - h) / 60.0
    } else {
        m1
    }) * 255.0
}

fn clamph(value: f64) -> f64 {
    let v = if value.is_nan() { 0.0 } else { value };
    let v = v.rem_euclid(360.0);
    if v < 0.0 { v + 360.0 } else { v }
}

fn clampt(value: f64) -> f64 {
    let v = if value.is_nan() { 0.0 } else { value };
    v.clamp(0.0, 1.0)
}
