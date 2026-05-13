//! Rust port of [d3-color](https://github.com/d3/d3-color).
//!
//! Provides parsing and conversion between common color spaces:
//! [`Rgb`], [`Hsl`], [`Lab`], [`Hcl`], and [`Cubehelix`].
//!
//! # Quick examples
//!
//! ```
//! use rgraph_color::{parse, Rgb, ColorSpace};
//!
//! let c = parse("#abcdef").unwrap();
//! assert_eq!(c.format_hex(), "#abcdef");
//! assert_eq!(c.format_rgb(), "rgb(171, 205, 239)");
//!
//! let r = parse("steelblue").unwrap().rgb();
//! let darker = r.darker(Some(1.0));
//! assert!(darker.r < r.r);
//! ```
//!
//! # NaN semantics
//!
//! d3-color uses `NaN` to mean "undefined channel" — for example, the hue of
//! a fully saturated black is undefined, and parsed transparent colors have
//! `NaN` r/g/b. This port preserves that contract: arithmetic with NaN
//! propagates, and formatting routines coerce NaN to `0` (channels) or `1`
//! (opacity), exactly like d3.
//!
//! # No external dependencies
//!
//! The crate is `std`-only with no third-party deps. The CSS color parser is
//! hand-written; the named-color table is `const` and binary-searched.

#![doc(html_no_source)]

pub mod color;
pub mod cubehelix;
pub mod hsl;
pub mod lab;
mod named;
mod parser;
pub mod rgb;

pub use color::{BRIGHTER, Color, ColorSpace, DARKER};
pub use cubehelix::Cubehelix;
pub use hsl::Hsl;
pub use lab::{Hcl, Lab, gray, lch};
pub use parser::parse;
pub use rgb::Rgb;

// Convenience constructors (mirrors d3's free functions).

/// Construct an [`Rgb`] from raw channel values. For parsing a string, use
/// [`parse`] or [`rgb_from_str`].
pub fn rgb(r: f64, g: f64, b: f64, opacity: f64) -> Rgb {
    Rgb::new(r, g, b, opacity)
}

/// Parse `format` as RGB. Equivalent to d3's `rgb(format)`.
pub fn rgb_from_str(format: &str) -> Rgb {
    match parse(format) {
        Some(c) => {
            let r = c.rgb();
            Rgb::new(r.r, r.g, r.b, r.opacity)
        }
        None => Rgb::empty(),
    }
}

/// Construct an [`Hsl`] from raw channel values.
pub fn hsl(h: f64, s: f64, l: f64, opacity: f64) -> Hsl {
    Hsl::new(h, s, l, opacity)
}

/// Convert any color to HSL. Equivalent to d3's `hsl(color)`.
pub fn hsl_from(c: &Color) -> Hsl {
    crate::hsl::from_color(c)
}

/// Parse `format` as HSL.
pub fn hsl_from_str(format: &str) -> Hsl {
    match parse(format) {
        Some(c) => crate::hsl::from_color(&c),
        None => Hsl::empty(),
    }
}

/// Construct a [`Lab`] from raw channel values.
pub fn lab(l: f64, a: f64, b: f64, opacity: f64) -> Lab {
    Lab::new(l, a, b, opacity)
}

/// Parse `format` as Lab.
pub fn lab_from_str(format: &str) -> Lab {
    match parse(format) {
        Some(c) => crate::lab::from_color(&c),
        None => Lab::empty(),
    }
}

/// Convert any color to Lab.
pub fn lab_from(c: &Color) -> Lab {
    crate::lab::from_color(c)
}

/// Construct an [`Hcl`] from raw channel values.
pub fn hcl(h: f64, c: f64, l: f64, opacity: f64) -> Hcl {
    Hcl::new(h, c, l, opacity)
}

/// Parse `format` as HCL.
pub fn hcl_from_str(format: &str) -> Hcl {
    match parse(format) {
        Some(c) => crate::lab::hcl_from_color(&c),
        None => Hcl::empty(),
    }
}

/// Convert any color to HCL.
pub fn hcl_from(c: &Color) -> Hcl {
    crate::lab::hcl_from_color(c)
}

/// Construct a [`Cubehelix`] from raw channel values.
pub fn cubehelix(h: f64, s: f64, l: f64, opacity: f64) -> Cubehelix {
    Cubehelix::new(h, s, l, opacity)
}

/// Convert any color to Cubehelix.
pub fn cubehelix_from(c: &Color) -> Cubehelix {
    crate::cubehelix::from_color(c)
}

/// Parse `format` as Cubehelix.
pub fn cubehelix_from_str(format: &str) -> Cubehelix {
    match parse(format) {
        Some(c) => crate::cubehelix::from_color(&c),
        None => Cubehelix::empty(),
    }
}

#[cfg(test)]
mod tests;
