//! RGB color space.

use core::fmt;

use crate::color::{BRIGHTER, Color, ColorSpace, DARKER};

/// RGB color in the sRGB space, with each channel held as a free-running
/// `f64` so that arithmetic preserves d3-color's "no clamp until you ask"
/// semantics. `r`, `g`, `b` are conceptually in `[0, 255]` but values
/// outside that range are valid and meaningful (e.g. as intermediates in
/// `brighter`/`darker`).
///
/// Channel values may be `NaN`, which signals "undefined" (d3 uses this to
/// represent fully transparent colors and unparseable inputs).
#[derive(Clone, Copy, Debug)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub opacity: f64,
}

impl Rgb {
    /// Construct a new RGB color. Inputs are taken as-is — no clamping, no
    /// rounding. Use [`Rgb::clamp`] to tighten values to displayable space.
    pub const fn new(r: f64, g: f64, b: f64, opacity: f64) -> Self {
        Rgb { r, g, b, opacity }
    }

    /// Constructs a new fully transparent RGB color (all channels NaN).
    pub fn empty() -> Self {
        Rgb { r: f64::NAN, g: f64::NAN, b: f64::NAN, opacity: f64::NAN }
    }

    /// Returns a copy with components rounded and clamped to `[0,255]` and
    /// opacity clamped to `[0,1]`. Equivalent to d3's `rgb.clamp()`.
    pub fn clamp(&self) -> Self {
        Rgb {
            r: clampi(self.r) as f64,
            g: clampi(self.g) as f64,
            b: clampi(self.b) as f64,
            opacity: clampa(self.opacity),
        }
    }

    /// `true` when all channels are within the displayable RGB gamut and
    /// opacity is within `[0, 1]`. NaN channels yield `false`.
    pub fn displayable(&self) -> bool {
        (-0.5..255.5).contains(&self.r)
            && (-0.5..255.5).contains(&self.g)
            && (-0.5..255.5).contains(&self.b)
            && (0.0..=1.0).contains(&self.opacity)
    }

    /// `#rrggbb` (six lowercase hex digits, alpha ignored).
    pub fn format_hex(&self) -> String {
        format!("#{}{}{}", hex_byte(self.r), hex_byte(self.g), hex_byte(self.b))
    }

    /// `#rrggbbaa`. Treats `NaN` opacity as 1.
    pub fn format_hex8(&self) -> String {
        let a = if self.opacity.is_nan() { 1.0 } else { self.opacity };
        format!(
            "#{}{}{}{}",
            hex_byte(self.r),
            hex_byte(self.g),
            hex_byte(self.b),
            hex_byte(a * 255.0),
        )
    }

    /// `rgb(r, g, b)` or `rgba(r, g, b, a)` exactly as d3 emits them.
    pub fn format_rgb(&self) -> String {
        let a = clampa(self.opacity);
        if (a - 1.0).abs() < f64::EPSILON {
            format!("rgb({}, {}, {})", clampi(self.r), clampi(self.g), clampi(self.b))
        } else {
            format!(
                "rgba({}, {}, {}, {})",
                clampi(self.r),
                clampi(self.g),
                clampi(self.b),
                fmt_num(a)
            )
        }
    }
}

impl ColorSpace for Rgb {
    fn rgb(&self) -> Rgb {
        *self
    }

    fn brighter(&self, k: Option<f64>) -> Self {
        let k = match k {
            None => BRIGHTER,
            Some(v) => BRIGHTER.powf(v),
        };
        Rgb::new(self.r * k, self.g * k, self.b * k, self.opacity)
    }

    fn darker(&self, k: Option<f64>) -> Self {
        let k = match k {
            None => DARKER,
            Some(v) => DARKER.powf(v),
        };
        Rgb::new(self.r * k, self.g * k, self.b * k, self.opacity)
    }

    fn into_color(self) -> Color {
        Color::Rgb(self)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_rgb())
    }
}

// ---------------------------------------------------------------------------
// Helpers — kept at module scope so they can be reused without exposing them.
// ---------------------------------------------------------------------------

/// Round to nearest integer and clamp to `[0,255]`. NaN becomes 0.
pub(crate) fn clampi(value: f64) -> i32 {
    if value.is_nan() {
        return 0;
    }
    let v = value.round();
    if v < 0.0 { 0 } else if v > 255.0 { 255 } else { v as i32 }
}

/// Clamp opacity to `[0,1]`. NaN becomes 1.
pub(crate) fn clampa(opacity: f64) -> f64 {
    if opacity.is_nan() {
        1.0
    } else {
        opacity.clamp(0.0, 1.0)
    }
}

fn hex_byte(value: f64) -> String {
    let v = clampi(value);
    if v < 16 {
        format!("0{:x}", v)
    } else {
        format!("{:x}", v)
    }
}

/// Format a float the way d3-color does (effectively JS' `String(x)`):
/// shortest round-trippable form. Rust's default `Display` for `f64` does
/// exactly this.
pub(crate) fn fmt_num(v: f64) -> String {
    format!("{}", v)
}
