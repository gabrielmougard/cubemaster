//! Cubehelix color space — Dave Green's perceptually uniform spiral
//! (<https://www.mrao.cam.ac.uk/~dag/CUBEHELIX/>).

use core::fmt;

use crate::color::{BRIGHTER, Color, ColorSpace, DARKER};
use crate::rgb::Rgb;

const A: f64 = -0.14861;
const B: f64 = 1.78277;
const C: f64 = -0.29227;
const D: f64 = -0.90649;
const E: f64 = 1.97294;

const RADIANS: f64 = core::f64::consts::PI / 180.0;
const DEGREES: f64 = 180.0 / core::f64::consts::PI;

#[derive(Clone, Copy, Debug)]
pub struct Cubehelix {
    pub h: f64,
    pub s: f64,
    pub l: f64,
    pub opacity: f64,
}

impl Cubehelix {
    pub const fn new(h: f64, s: f64, l: f64, opacity: f64) -> Self {
        Cubehelix { h, s, l, opacity }
    }

    pub fn empty() -> Self {
        Cubehelix { h: f64::NAN, s: f64::NAN, l: f64::NAN, opacity: f64::NAN }
    }
}

impl ColorSpace for Cubehelix {
    fn rgb(&self) -> Rgb {
        let h = if self.h.is_nan() { 0.0 } else { (self.h + 120.0) * RADIANS };
        let l = self.l;
        let a = if self.s.is_nan() { 0.0 } else { self.s * l * (1.0 - l) };
        let cosh = h.cos();
        let sinh = h.sin();
        Rgb::new(
            255.0 * (l + a * (A * cosh + B * sinh)),
            255.0 * (l + a * (C * cosh + D * sinh)),
            255.0 * (l + a * (E * cosh)),
            self.opacity,
        )
    }

    fn brighter(&self, k: Option<f64>) -> Self {
        let k = match k {
            None => BRIGHTER,
            Some(v) => BRIGHTER.powf(v),
        };
        Cubehelix::new(self.h, self.s, self.l * k, self.opacity)
    }

    fn darker(&self, k: Option<f64>) -> Self {
        let k = match k {
            None => DARKER,
            Some(v) => DARKER.powf(v),
        };
        Cubehelix::new(self.h, self.s, self.l * k, self.opacity)
    }

    fn into_color(self) -> Color {
        Color::Cubehelix(self)
    }
}

impl fmt::Display for Cubehelix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.rgb().format_rgb())
    }
}

/// Convert any color into Cubehelix. Mirrors d3's `cubehelixConvert`.
pub fn from_color(c: &Color) -> Cubehelix {
    if let Color::Cubehelix(ch) = c {
        return *ch;
    }
    let o = c.rgb();
    let r = o.r / 255.0;
    let g = o.g / 255.0;
    let b = o.b / 255.0;

    // Pre-compute the constants used to project (r, g, b) onto the helix axis.
    let ed = E * D;
    let eb = E * B;
    let bc_da = B * C - D * A;

    let l = (bc_da * b + ed * r - eb * g) / (bc_da + ed - eb);
    let bl = b - l;
    let k = (E * (g - l) - C * bl) / D;
    let denom = E * l * (1.0 - l);
    let s = if denom == 0.0 {
        f64::NAN
    } else {
        (k * k + bl * bl).sqrt() / denom
    };
    let h = if s != 0.0 && !s.is_nan() {
        let h = k.atan2(bl) * DEGREES - 120.0;
        if h < 0.0 { h + 360.0 } else { h }
    } else {
        f64::NAN
    };
    Cubehelix::new(h, s, l, o.opacity)
}
