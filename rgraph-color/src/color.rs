//! Common color types and traits.
//!
//! [`Color`] is the dynamic dispatch enum returned by the parser and lets
//! callers route a value through any of the typed color spaces. [`ColorSpace`]
//! is the per-space trait every concrete struct implements (`Rgb`, `Hsl`,
//! `Lab`, `Hcl`, `Cubehelix`).

use core::fmt;

use crate::cubehelix::Cubehelix;
use crate::hsl::Hsl;
use crate::lab::{Hcl, Lab};
use crate::rgb::Rgb;

/// d3-color's `darker` / `brighter` constants.
pub const DARKER: f64 = 0.7;
/// 1 / DARKER ≈ 1.4285714…
pub const BRIGHTER: f64 = 1.0 / DARKER;

/// Behaviour shared by all concrete color-space types.
///
/// Implementors are typically `Copy`, so methods take `&self` and return
/// owned values.
pub trait ColorSpace: Sized + Copy {
    /// Convert into [`Rgb`].
    fn rgb(&self) -> Rgb;

    /// Brighten by `k` (default `1.0` when `None`). Each space uses its own
    /// formula — see d3-color's docs.
    fn brighter(&self, k: Option<f64>) -> Self;

    /// Darken by `k` (default `1.0` when `None`).
    fn darker(&self, k: Option<f64>) -> Self;

    /// Lift into the dynamic [`Color`] enum.
    fn into_color(self) -> Color;
}

/// Discriminated union over every supported color space.
///
/// The parser produces [`Color::Rgb`] or [`Color::Hsl`]; the typed
/// conversion helpers (`Lab::from`, `Hcl::from`, `Cubehelix::from`) lift to
/// the corresponding variants. Use [`Color::rgb`] for a uniform projection.
#[derive(Clone, Copy, Debug)]
pub enum Color {
    Rgb(Rgb),
    Hsl(Hsl),
    Lab(Lab),
    Hcl(Hcl),
    Cubehelix(Cubehelix),
}

impl Color {
    /// Convert to [`Rgb`] regardless of the underlying variant.
    pub fn rgb(&self) -> Rgb {
        match self {
            Color::Rgb(c) => c.rgb(),
            Color::Hsl(c) => c.rgb(),
            Color::Lab(c) => c.rgb(),
            Color::Hcl(c) => c.rgb(),
            Color::Cubehelix(c) => c.rgb(),
        }
    }

    /// `displayable()` from d3 — checks if the corresponding RGB is
    /// representable in the standard sRGB gamut with opacity in `[0,1]`.
    pub fn displayable(&self) -> bool {
        self.rgb().displayable()
    }

    /// `#rrggbb` — alpha is ignored.
    pub fn format_hex(&self) -> String {
        self.rgb().format_hex()
    }

    /// `#rrggbbaa`.
    pub fn format_hex8(&self) -> String {
        self.rgb().format_hex8()
    }

    /// `rgb(...)` / `rgba(...)`.
    pub fn format_rgb(&self) -> String {
        self.rgb().format_rgb()
    }

    /// `hsl(...)` / `hsla(...)`.
    pub fn format_hsl(&self) -> String {
        crate::hsl::from_color(self).format_hsl()
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // d3's `Color.toString` uses RGB.
        f.write_str(&self.format_rgb())
    }
}

impl From<Rgb> for Color {
    fn from(v: Rgb) -> Self { Color::Rgb(v) }
}
impl From<Hsl> for Color {
    fn from(v: Hsl) -> Self { Color::Hsl(v) }
}
impl From<Lab> for Color {
    fn from(v: Lab) -> Self { Color::Lab(v) }
}
impl From<Hcl> for Color {
    fn from(v: Hcl) -> Self { Color::Hcl(v) }
}
impl From<Cubehelix> for Color {
    fn from(v: Cubehelix) -> Self { Color::Cubehelix(v) }
}
