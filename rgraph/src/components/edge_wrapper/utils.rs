//! Port of `xyflow-react/src/components/EdgeWrapper/utils.ts`.
//!
//! Status: Phase 6 — implemented.
//!
//! Built-in edge variants resolved by string identifier. Maps to the
//! TS `builtinEdgeTypes` record but as an enum (so the match in
//! `EdgeWrapper` stays exhaustive).

#![allow(clippy::module_name_repetitions)]

/// Identifier for one of the five built-in edge renderers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInEdgeType {
    Default,
    Straight,
    Step,
    SmoothStep,
    SimpleBezier,
}

impl BuiltInEdgeType {
    /// Parse from the TS-style string identifier.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "default" => Some(BuiltInEdgeType::Default),
            "straight" => Some(BuiltInEdgeType::Straight),
            "step" => Some(BuiltInEdgeType::Step),
            "smoothstep" => Some(BuiltInEdgeType::SmoothStep),
            "simplebezier" => Some(BuiltInEdgeType::SimpleBezier),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BuiltInEdgeType::Default => "default",
            BuiltInEdgeType::Straight => "straight",
            BuiltInEdgeType::Step => "step",
            BuiltInEdgeType::SmoothStep => "smoothstep",
            BuiltInEdgeType::SimpleBezier => "simplebezier",
        }
    }
}

/// Null-position sentinel returned when an edge's source/target node
/// can't be resolved. Mirrors the TS `nullPosition` object.
#[derive(Debug, Clone, Copy)]
pub struct NullPosition;

impl NullPosition {
    pub const SOURCE_X: Option<f64> = None;
    pub const SOURCE_Y: Option<f64> = None;
    pub const TARGET_X: Option<f64> = None;
    pub const TARGET_Y: Option<f64> = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for s in ["default", "straight", "step", "smoothstep", "simplebezier"] {
            let t = BuiltInEdgeType::parse(s).unwrap();
            assert_eq!(t.as_str(), s);
        }
        assert!(BuiltInEdgeType::parse("custom").is_none());
    }
}
