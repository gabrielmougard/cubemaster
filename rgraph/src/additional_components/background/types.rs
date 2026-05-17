//! Port of `xyflow-react/src/additional-components/Background/types.ts`.
//!
//! Status: Phase 8 — implemented.

#![allow(clippy::module_name_repetitions)]

/// The three pattern variants supported by [`super::background::Background`].
///
/// Mirrors the TS `BackgroundVariant` enum. Encoded as strings via
/// [`Self::as_str`] for class-name composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackgroundVariant {
    Lines,
    #[default]
    Dots,
    Cross,
}

impl BackgroundVariant {
    /// `"lines"` / `"dots"` / `"cross"`. Used in the CSS class
    /// `react-flow__background-pattern <variant>`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BackgroundVariant::Lines => "lines",
            BackgroundVariant::Dots => "dots",
            BackgroundVariant::Cross => "cross",
        }
    }
}

/// Gap accepts a single value (square gap) or a `(gap_x, gap_y)` pair.
/// Mirrors the TS `gap?: number | [number, number]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackgroundGap {
    Uniform(f64),
    Sided(f64, f64),
}

impl Default for BackgroundGap {
    fn default() -> Self {
        BackgroundGap::Uniform(20.0)
    }
}

impl BackgroundGap {
    pub fn as_tuple(&self) -> (f64, f64) {
        match self {
            BackgroundGap::Uniform(v) => (*v, *v),
            BackgroundGap::Sided(x, y) => (*x, *y),
        }
    }
}

impl From<f64> for BackgroundGap {
    fn from(v: f64) -> Self {
        BackgroundGap::Uniform(v)
    }
}

impl From<(f64, f64)> for BackgroundGap {
    fn from(v: (f64, f64)) -> Self {
        BackgroundGap::Sided(v.0, v.1)
    }
}

/// Same shape for `offset`.
pub type BackgroundOffset = BackgroundGap;
